use super::*;
use anyhow::{ensure, Context};
use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupFile {
    scope: String,
    path: String,
    asset: Option<String>,
    role: Option<String>,
    bytes: u64,
    modified_ms: u128,
    hash: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    format: u32,
    id: String,
    location_id: String,
    location_name: String,
    source_path: String,
    created_at: String,
    database_hash: String,
    files: Vec<BackupFile>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSnapshot {
    pub id: String,
    pub location_id: String,
    pub location_name: String,
    pub created_at: String,
    pub file_count: usize,
    pub byte_count: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub snapshot: BackupSnapshot,
    pub copied: usize,
    pub reused: usize,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupConflict {
    pub path: String,
    pub backup_bytes: u64,
    pub current_bytes: u64,
    pub backup_modified_ms: u128,
    pub current_modified_ms: u128,
    pub backup_newer: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePlan {
    pub snapshot: BackupSnapshot,
    pub conflicts: Vec<BackupConflict>,
    pub missing: usize,
    pub unchanged: usize,
    pub target_path: String,
    pub token: String,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestorePolicy {
    Newer,
    Skip,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub restored: usize,
    pub skipped: usize,
    pub library: Library,
}

fn modified(path: &Path) -> Result<u128> {
    Ok(fs::metadata(path)?
        .modified()?
        .duration_since(UNIX_EPOCH)?
        .as_millis())
}
fn digest(path: &Path) -> Result<String> {
    let mut input = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
fn blob_path(repo: &Path, hash: &str) -> Result<PathBuf> {
    ensure!(
        hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_hexdigit()),
        "备份文件校验值无效"
    );
    Ok(repo.join("blobs").join(&hash[..2]).join(hash))
}
fn store_blob(repo: &Path, source: &Path) -> Result<(String, bool)> {
    let hash = digest(source)?;
    let dest = blob_path(repo, &hash)?;
    if dest.exists() {
        ensure!(
            digest(&dest)? == hash,
            "备份仓库已有数据损坏：{}",
            dest.display()
        );
        return Ok((hash, false));
    }
    fs::create_dir_all(dest.parent().unwrap())?;
    let part = dest.with_extension(format!("{}.part", id()));
    let result = (|| -> Result<()> {
        let mut input = File::open(source)?;
        let mut output = File::create(&part)?;
        std::io::copy(&mut input, &mut output)?;
        output.sync_all()?;
        ensure!(digest(&part)? == hash, "备份期间文件发生变化");
        fs::rename(&part, &dest)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&part);
    }
    result?;
    Ok((hash, true))
}
fn safe_relative(text: &str) -> Result<&Path> {
    let path = Path::new(text);
    ensure!(
        !text.is_empty() && path.components().all(|c| matches!(c, Component::Normal(_))),
        "备份中的文件路径无效"
    );
    Ok(path)
}
fn safe_target(root: &Path, relative: &str) -> Result<PathBuf> {
    let rel = safe_relative(relative)?;
    let mut at = root.to_path_buf();
    for part in rel.components() {
        at.push(part.as_os_str());
        if at.is_symlink() {
            bail!("目标路径包含符号链接：{}", at.display());
        }
    }
    Ok(at)
}
fn snapshot_of(m: &Manifest) -> BackupSnapshot {
    BackupSnapshot {
        id: m.id.clone(),
        location_id: m.location_id.clone(),
        location_name: m.location_name.clone(),
        created_at: m.created_at.clone(),
        file_count: m.files.len(),
        byte_count: m.files.iter().map(|f| f.bytes).sum(),
    }
}
fn manifest_path(repo: &Path, snapshot: &str) -> Result<PathBuf> {
    ensure!(
        !snapshot.is_empty()
            && snapshot
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-'),
        "备份快照编号无效"
    );
    Ok(repo.join("snapshots").join(format!("{snapshot}.json")))
}
fn load_manifest(repo: &Path, snapshot: &str) -> Result<Manifest> {
    let m: Manifest = serde_json::from_reader(File::open(manifest_path(repo, snapshot)?)?)?;
    ensure!(m.format == 1 && m.id == snapshot, "不支持的备份格式");
    blob_path(repo, &m.database_hash)?;
    for f in &m.files {
        safe_relative(&f.path)?;
        blob_path(repo, &f.hash)?;
        ensure!(
            f.scope == "local" || f.scope == "location",
            "备份文件范围无效"
        );
    }
    Ok(m)
}
fn walk_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() || entry.file_name().to_string_lossy().starts_with(".cheki-") {
            continue;
        }
        if kind.is_dir() {
            walk_files(root, &entry.path(), out)?;
        } else if kind.is_file() {
            out.push(entry.path().strip_prefix(root)?.to_path_buf());
        }
    }
    Ok(())
}
impl Store {
    pub fn backup_list(&self, repo: &Path) -> Result<Vec<BackupSnapshot>> {
        let dir = repo.join("snapshots");
        if !dir.is_dir() {
            return Ok(vec![]);
        }
        let mut result = vec![];
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.path().extension().is_some_and(|e| e == "json") {
                let id = entry
                    .path()
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                result.push(snapshot_of(&load_manifest(repo, &id)?));
            }
        }
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(result)
    }
    pub fn backup_location(
        &mut self,
        location_id: &str,
        repo: &Path,
        mut progress: impl FnMut(usize, usize, &str),
    ) -> Result<BackupResult> {
        let location = self
            .locations()?
            .into_iter()
            .find(|l| l.id == location_id)
            .context("找不到目录")?;
        ensure!(location.online, "目录离线，无法备份");
        let source_root = PathBuf::from(&location.path);
        fs::create_dir_all(repo)?;
        let repo = fs::canonicalize(repo)?;
        ensure!(
            !repo.starts_with(&source_root)
                && !source_root.starts_with(&repo)
                && !repo.starts_with(&self.root)
                && !self.root.starts_with(&repo),
            "备份目录不能位于图库或原件目录中"
        );
        let linked: HashMap<(String, String), (String, String)> = self
            .db
            .prepare("SELECT location_id,relative_path,asset_id,role FROM renditions")?
            .query_map([], |r| Ok(((r.get(0)?, r.get(1)?), (r.get(2)?, r.get(3)?))))?
            .collect::<rusqlite::Result<_>>()?;
        let mut input: Vec<(String, PathBuf, PathBuf)> = vec![];
        if location_id == "local" {
            for folder in ["originals", "previews", "people"] {
                let mut files = vec![];
                walk_files(&self.root, &self.root.join(folder), &mut files)?;
                for rel in files {
                    input.push(("local".into(), rel, self.root.clone()));
                }
            }
        } else {
            for ((loc, path), (_, role)) in &linked {
                if loc == location_id && role == "original" {
                    input.push(("location".into(), PathBuf::from(path), source_root.clone()));
                }
            }
            let external_assets: HashSet<&str> = linked
                .iter()
                .filter(|((l, _), (_, r))| l == location_id && r == "original")
                .map(|(_, (a, _))| a.as_str())
                .collect();
            for ((loc, path), (asset, role)) in &linked {
                if loc == "local"
                    && ["base", "display"].contains(&role.as_str())
                    && external_assets.contains(asset.as_str())
                {
                    input.push(("local".into(), PathBuf::from(path), self.root.clone()));
                }
            }
        }
        input.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
        input.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
        let mut files = Vec::with_capacity(input.len());
        let (mut copied, mut reused) = (0, 0);
        for (i, (scope, rel, root)) in input.iter().enumerate() {
            let path = safe_target(root, &rel.to_string_lossy())?;
            ensure!(path.is_file(), "备份源文件不可用：{}", path.display());
            let meta = fs::metadata(&path)?;
            let key = (scope.clone(), rel.to_string_lossy().to_string());
            let before = modified(&path)?;
            // Hash each source even when size and mtime match: external edits can
            // preserve both timestamps, while blob reuse remains incremental.
            let (hash, new) = store_blob(&repo, &path)?;
            if new {
                copied += 1;
            } else {
                reused += 1;
            }
            ensure!(
                fs::metadata(&path)?.len() == meta.len() && modified(&path)? == before,
                "备份期间源文件发生变化：{}",
                path.display()
            );
            let pair = linked.get(&(
                if scope == "local" {
                    "local"
                } else {
                    location_id
                }
                .to_string(),
                key.1.clone(),
            ));
            files.push(BackupFile {
                scope: scope.clone(),
                path: key.1,
                asset: pair.map(|p| p.0.clone()),
                role: pair.map(|p| p.1.clone()),
                bytes: meta.len(),
                modified_ms: before,
                hash,
            });
            progress(
                i + 1,
                input.len(),
                &path.file_name().unwrap_or_default().to_string_lossy(),
            );
        }
        let db_part = repo.join(format!(".database-{}.sqlite", id()));
        let db_result = self
            .db
            .execute("VACUUM INTO ?1", [db_part.to_string_lossy().as_ref()]);
        if let Err(e) = db_result {
            let _ = fs::remove_file(&db_part);
            return Err(e.into());
        }
        let (database_hash, new) = store_blob(&repo, &db_part)?;
        if new {
            copied += 1;
        } else {
            reused += 1;
        }
        fs::remove_file(db_part)?;
        fs::create_dir_all(repo.join("snapshots"))?;
        let manifest = Manifest {
            format: 1,
            id: id(),
            location_id: location.id,
            location_name: location.name,
            source_path: location.path,
            created_at: chrono::Utc::now().to_rfc3339(),
            database_hash,
            files,
        };
        let path = manifest_path(&repo, &manifest.id)?;
        let part = path.with_extension("part");
        let mut file = File::create(&part)?;
        serde_json::to_writer(&mut file, &manifest)?;
        file.sync_all()?;
        fs::rename(part, path)?;
        Ok(BackupResult {
            snapshot: snapshot_of(&manifest),
            copied,
            reused,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn picture(path: &Path, color: [u8; 3]) {
        image::RgbImage::from_pixel(56, 84, image::Rgb(color))
            .save(path)
            .unwrap();
    }
    #[test]
    fn incremental_backup_and_conflict_choices() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("library");
        let repo = tmp.path().join("backup");
        let input = tmp.path().join("photo.png");
        picture(&input, [230, 120, 30]);
        let mut s = Store::open(root).unwrap();
        let original = s.import(vec![input], None).unwrap().library.chekis[0].assets[0]
            .original_path
            .clone();
        let first = s.backup_location("local", &repo, |_, _, _| {}).unwrap();
        let second = s.backup_location("local", &repo, |_, _, _| {}).unwrap();
        assert_eq!(first.snapshot.file_count, second.snapshot.file_count);
        assert_eq!(second.copied, 0);
        assert!(second.reused > 0);
        picture(Path::new(&original), [20, 170, 240]);
        let plan = s.restore_plan(&repo, &first.snapshot.id, None).unwrap();
        assert_eq!(plan.conflicts.len(), 1);
        picture(Path::new(&original), [21, 171, 241]);
        assert!(s
            .restore_backup(
                &repo,
                &first.snapshot.id,
                None,
                RestorePolicy::Newer,
                &plan.token,
                |_, _, _| {}
            )
            .is_err());
        assert_eq!(
            image::open(&original).unwrap().to_rgb8().get_pixel(0, 0).0,
            [21, 171, 241]
        );
        picture(Path::new(&original), [20, 170, 240]);
        let plan = s.restore_plan(&repo, &first.snapshot.id, None).unwrap();
        let skipped = s
            .restore_backup(
                &repo,
                &first.snapshot.id,
                None,
                RestorePolicy::Skip,
                &plan.token,
                |_, _, _| {},
            )
            .unwrap();
        assert_eq!(skipped.restored, 0);
        assert_eq!(
            image::open(&original).unwrap().to_rgb8().get_pixel(0, 0).0,
            [20, 170, 240]
        );
        // The backup's mtime is deliberately made later than the current file's mtime.
        let manifest_path = manifest_path(&repo, &first.snapshot.id).unwrap();
        let mut manifest: Manifest =
            serde_json::from_reader(File::open(&manifest_path).unwrap()).unwrap();
        manifest
            .files
            .iter_mut()
            .find(|f| f.role.as_deref() == Some("original"))
            .unwrap()
            .modified_ms = modified(Path::new(&original)).unwrap() + 60_000;
        serde_json::to_writer(File::create(&manifest_path).unwrap(), &manifest).unwrap();
        let plan = s.restore_plan(&repo, &first.snapshot.id, None).unwrap();
        let restored = s
            .restore_backup(
                &repo,
                &first.snapshot.id,
                None,
                RestorePolicy::Newer,
                &plan.token,
                |_, _, _| {},
            )
            .unwrap();
        assert_eq!(restored.restored, 1);
        assert_eq!(
            image::open(&original).unwrap().to_rgb8().get_pixel(0, 0).0,
            [230, 120, 30]
        );
    }
    #[test]
    fn restore_into_empty_library_recovers_catalog_and_files() {
        let tmp = tempfile::tempdir().unwrap();
        let input = tmp.path().join("photo.png");
        picture(&input, [101, 121, 141]);
        let repo = tmp.path().join("backup");
        let mut source = Store::open(tmp.path().join("source")).unwrap();
        source.import(vec![input], None).unwrap();
        let snap = source
            .backup_location("local", &repo, |_, _, _| {})
            .unwrap()
            .snapshot;
        let mut dest = Store::open(tmp.path().join("dest")).unwrap();
        let plan = dest.restore_plan(&repo, &snap.id, None).unwrap();
        assert!(plan.missing >= 2);
        assert!(plan.conflicts.is_empty());
        let result = dest
            .restore_backup(
                &repo,
                &snap.id,
                None,
                RestorePolicy::Skip,
                &plan.token,
                |_, _, _| {},
            )
            .unwrap();
        assert_eq!(result.library.chekis.len(), 1);
        assert!(Path::new(&result.library.chekis[0].assets[0].original_path).is_file());
    }
    #[test]
    fn external_snapshot_restores_to_new_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let source_disk = tmp.path().join("disk-a");
        fs::create_dir(&source_disk).unwrap();
        let input = source_disk.join("photo.png");
        picture(&input, [70, 80, 90]);
        let repo = tmp.path().join("backup");
        let mut source = Store::open(tmp.path().join("source")).unwrap();
        source.add_location(&source_disk, None).unwrap();
        source
            .import_file(
                &input,
                &ImportOptions {
                    reference: true,
                    ..Default::default()
                },
            )
            .unwrap();
        let location = source
            .locations()
            .unwrap()
            .into_iter()
            .find(|l| l.id != "local")
            .unwrap();
        let snap = source
            .backup_location(&location.id, &repo, |_, _, _| {})
            .unwrap()
            .snapshot;
        let target = tmp.path().join("disk-b");
        fs::create_dir(&target).unwrap();
        let mut dest = Store::open(tmp.path().join("dest")).unwrap();
        let plan = dest.restore_plan(&repo, &snap.id, Some(&target)).unwrap();
        assert_eq!(plan.missing, snap.file_count);
        let result = dest
            .restore_backup(
                &repo,
                &snap.id,
                Some(&target),
                RestorePolicy::Skip,
                &plan.token,
                |_, _, _| {},
            )
            .unwrap();
        assert_eq!(result.library.chekis.len(), 1);
        let a = &result.library.chekis[0].assets[0];
        assert!(a.original_path.starts_with(target.to_str().unwrap()));
        assert!(Path::new(&a.original_path).is_file());
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct RestoreOp {
    destination: PathBuf,
    previous: PathBuf,
    staged: PathBuf,
    had_file: bool,
}
#[derive(Serialize, Deserialize)]
struct RestoreJournal {
    id: String,
    ops: Vec<RestoreOp>,
}
struct Candidate {
    file: BackupFile,
    dest: PathBuf,
    current_hash: Option<String>,
    current_modified_ms: Option<u128>,
    conflict: bool,
    backup_newer: bool,
    unchanged: bool,
}
impl Store {
    fn restore_root(&self, m: &Manifest, target: Option<&Path>) -> Result<PathBuf> {
        if m.location_id == "local" {
            return Ok(self.root.clone());
        }
        let current: Option<String> = self
            .db
            .query_row(
                "SELECT path FROM locations WHERE id=?1",
                [&m.location_id],
                |r| r.get(0),
            )
            .optional()?;
        let chosen = target
            .map(Path::to_path_buf)
            .or_else(|| current.as_ref().map(PathBuf::from))
            .context("请指定外置目录的恢复位置")?;
        let chosen = fs::canonicalize(chosen).context("恢复目标目录不可用")?;
        ensure!(
            chosen.is_dir() && !chosen.starts_with(&self.root) && !self.root.starts_with(&chosen),
            "外置恢复目录不能与本机图库重叠"
        );
        if let Some(c) = current {
            ensure!(
                Path::new(&c) == chosen || !Path::new(&c).exists(),
                "已登记的外置目录在线，请恢复到当前登记的路径"
            );
        }
        Ok(chosen)
    }
    fn candidates(
        &self,
        repo: &Path,
        m: &Manifest,
        target: Option<&Path>,
    ) -> Result<(PathBuf, Vec<Candidate>)> {
        let external = self.restore_root(m, target)?;
        let mut result = vec![];
        let mut destinations = HashSet::new();
        for f in &m.files {
            let (root, location) = if f.scope == "local" {
                (&self.root, "local")
            } else {
                (&external, m.location_id.as_str())
            };
            let relative = if let (Some(asset), Some(role)) = (&f.asset, &f.role) {
                self.db.query_row("SELECT relative_path FROM renditions WHERE asset_id=?1 AND role=?2 AND location_id=?3",params![asset,role,location],|r|r.get::<_,String>(0)).optional()?.unwrap_or_else(|| f.path.clone())
            } else {
                f.path.clone()
            };
            let dest = safe_target(root, &relative)?;
            ensure!(
                dest != self.root.join("library.sqlite") && destinations.insert(dest.clone()),
                "备份文件目标重复"
            );
            let exists = dest.is_file();
            let current_hash = if exists { Some(digest(&dest)?) } else { None };
            let current_modified_ms = if exists { Some(modified(&dest)?) } else { None };
            let unchanged = exists && current_hash.as_deref() == Some(f.hash.as_str());
            let conflict = exists && !unchanged;
            let backup_newer = conflict && f.modified_ms > current_modified_ms.unwrap_or(0);
            ensure!(
                blob_path(repo, &f.hash)?.is_file(),
                "备份数据缺失：{}",
                f.path
            );
            result.push(Candidate {
                file: f.clone(),
                dest,
                current_hash,
                current_modified_ms,
                conflict,
                backup_newer,
                unchanged,
            });
        }
        Ok((external, result))
    }
    pub fn restore_plan(
        &self,
        repo: &Path,
        snapshot: &str,
        target: Option<&Path>,
    ) -> Result<RestorePlan> {
        let m = load_manifest(repo, snapshot)?;
        ensure!(
            blob_path(repo, &m.database_hash)?.is_file(),
            "备份数据库缺失"
        );
        let (external, candidates) = self.candidates(repo, &m, target)?;
        ensure!(
            !external.starts_with(repo) && !repo.starts_with(&external),
            "恢复目标不能与备份目录重叠"
        );
        let target_path = external.to_string_lossy().into_owned();
        let state: Vec<_> = candidates
            .iter()
            .map(|c| (&c.dest, &c.current_hash, c.current_modified_ms))
            .collect();
        let token = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(&m.files, &state, &target_path))?)
        );
        let mut conflicts = vec![];
        let mut missing = 0;
        let mut unchanged = 0;
        for c in candidates {
            if c.unchanged {
                unchanged += 1;
            } else if c.conflict {
                conflicts.push(BackupConflict {
                    path: format!("{} / {}", c.file.scope, c.file.path),
                    backup_bytes: c.file.bytes,
                    current_bytes: fs::metadata(&c.dest)?.len(),
                    backup_modified_ms: c.file.modified_ms,
                    current_modified_ms: c.current_modified_ms.unwrap_or(0),
                    backup_newer: c.backup_newer,
                });
            } else {
                missing += 1;
            }
        }
        Ok(RestorePlan {
            snapshot: snapshot_of(&m),
            conflicts,
            missing,
            unchanged,
            target_path,
            token,
        })
    }
    pub(super) fn recover_restores(&mut self) -> Result<()> {
        fs::create_dir_all(self.root.join("restores"))?;
        for entry in fs::read_dir(self.root.join("restores"))? {
            let path = entry?.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let journal: RestoreJournal = serde_json::from_reader(File::open(&path)?)?;
            let committed: bool = self.db.query_row(
                "SELECT EXISTS(SELECT 1 FROM restore_runs WHERE id=?1)",
                [&journal.id],
                |r| r.get(0),
            )?;
            for op in journal.ops.iter().rev() {
                // Journal names are generated by the app; never follow a swapped symlink.
                ensure!(
                    !op.destination.is_symlink()
                        && !op.previous.is_symlink()
                        && !op.staged.is_symlink(),
                    "恢复路径包含符号链接"
                );
                if committed {
                    if op.previous.is_file() {
                        fs::remove_file(&op.previous)?;
                    }
                } else if op.previous.is_file() {
                    if op.destination.is_file() {
                        fs::remove_file(&op.destination)?;
                    }
                    fs::rename(&op.previous, &op.destination)?;
                } else if !op.had_file && op.destination.is_file() {
                    fs::remove_file(&op.destination)?;
                }
                if op.staged.is_file() {
                    fs::remove_file(&op.staged)?;
                }
            }
            fs::remove_file(path)?;
            if committed {
                self.db
                    .execute("DELETE FROM restore_runs WHERE id=?1", [journal.id])?;
            }
        }
        Ok(())
    }
    pub fn restore_backup(
        &mut self,
        repo: &Path,
        snapshot: &str,
        target: Option<&Path>,
        policy: RestorePolicy,
        expected_token: &str,
        mut progress: impl FnMut(usize, usize, &str),
    ) -> Result<RestoreResult> {
        self.recover_restores()?;
        ensure!(
            self.restore_plan(repo, snapshot, target)?.token == expected_token,
            "文件状态已变化，请重新预览恢复冲突"
        );
        let m = load_manifest(repo, snapshot)?;
        ensure!(
            digest(&blob_path(repo, &m.database_hash)?)? == m.database_hash,
            "备份数据库校验失败"
        );
        let (external, candidates) = self.candidates(repo, &m, target)?;
        ensure!(
            !external.starts_with(repo) && !repo.starts_with(&external),
            "恢复目标不能与备份目录重叠"
        );
        let total = candidates.len();
        let selected: Vec<Candidate> = candidates
            .into_iter()
            .filter(|c| {
                !c.unchanged
                    && (!c.conflict || matches!(policy, RestorePolicy::Newer) && c.backup_newer)
            })
            .collect();
        let skipped = total - selected.len();
        // Stage and verify every blob before moving a single current file.
        let run = id();
        let mut ops: Vec<RestoreOp> = vec![];
        for (i, c) in selected.iter().enumerate() {
            let blob = blob_path(repo, &c.file.hash)?;
            fs::create_dir_all(c.dest.parent().unwrap())?;
            let staged = c
                .dest
                .with_file_name(format!(".cheki-restore-{run}-{i}.part"));
            let previous = c
                .dest
                .with_file_name(format!(".cheki-restore-{run}-{i}.previous"));
            let copy = (|| -> Result<()> {
                let mut input = File::open(blob)?;
                let mut output = File::create(&staged)?;
                std::io::copy(&mut input, &mut output)?;
                output.sync_all()?;
                ensure!(
                    digest(&staged)? == c.file.hash,
                    "备份文件校验失败：{}",
                    c.file.path
                );
                ensure!(
                    c.file.modified_ms <= i64::MAX as u128 * 1000,
                    "备份文件时间无效"
                );
                filetime::set_file_mtime(
                    &staged,
                    filetime::FileTime::from_unix_time(
                        (c.file.modified_ms / 1000) as i64,
                        ((c.file.modified_ms % 1000) * 1_000_000) as u32,
                    ),
                )?;
                Ok(())
            })();
            if let Err(e) = copy {
                let _ = fs::remove_file(&staged);
                for op in &ops {
                    let _ = fs::remove_file(&op.staged);
                }
                return Err(e);
            }
            ops.push(RestoreOp {
                destination: c.dest.clone(),
                previous,
                staged,
                had_file: c.dest.is_file(),
            });
            progress(i + 1, total, &c.file.path);
        }
        let journal = RestoreJournal {
            id: run.clone(),
            ops,
        };
        let journalpath = self.root.join("restores").join(format!("{run}.json"));
        let journalpart = journalpath.with_extension("part");
        let mut record = File::create(&journalpart)?;
        serde_json::to_writer(&mut record, &journal)?;
        record.sync_all()?;
        fs::rename(journalpart, &journalpath)?;
        let result = (|| -> Result<()> {
            for op in &journal.ops {
                if op.had_file {
                    fs::rename(&op.destination, &op.previous)?;
                }
                fs::rename(&op.staged, &op.destination)?;
            }
            // The catalog snapshot is an SQLite-consistent copy. Merge only this location's
            // records; existing collections and their manually edited metadata win.
            self.merge_snapshot(
                &m,
                &blob_path(repo, &m.database_hash)?,
                &external,
                &selected,
                &run,
            )?;
            Ok(())
        })();
        if result.is_err() {
            let _ = self.recover_restores();
            result?;
        }
        self.recover_restores()?;
        let restored = selected.len();
        Ok(RestoreResult {
            restored,
            skipped,
            library: self.list()?,
        })
    }
    fn merge_snapshot(
        &mut self,
        m: &Manifest,
        dbfile: &Path,
        external: &Path,
        selected: &[Candidate],
        run: &str,
    ) -> Result<()> {
        self.db.execute(
            "ATTACH DATABASE ?1 AS source",
            [dbfile.to_string_lossy().as_ref()],
        )?;
        let result = (|| -> Result<()> {
            let source_version: i64 = self
                .db
                .query_row("PRAGMA source.user_version", [], |r| r.get(0))?;
            ensure!(source_version == 5, "备份数据库版本不兼容");
            let tx = self.db.transaction()?;
            if m.location_id != "local" {
                tx.execute(
                    "INSERT OR IGNORE INTO locations(id,path,name) VALUES(?1,?2,?3)",
                    params![m.location_id, external.to_string_lossy(), m.location_name],
                )?;
                tx.execute(
                    "UPDATE locations SET path=?2 WHERE id=?1",
                    params![m.location_id, external.to_string_lossy()],
                )?;
            }
            tx.execute("CREATE TEMP TABLE restore_assets AS SELECT DISTINCT asset_id AS id FROM source.renditions WHERE location_id=?1 AND role='original'",[&m.location_id])?;
            tx.execute_batch("CREATE TEMP TABLE restore_chekis AS SELECT DISTINCT cheki_id AS id FROM source.cheki_assets WHERE asset_id IN (SELECT id FROM restore_assets);
                CREATE TEMP TABLE new_chekis AS SELECT id FROM restore_chekis WHERE id NOT IN (SELECT id FROM main.chekis);
                INSERT OR IGNORE INTO people SELECT * FROM source.people;
                INSERT OR IGNORE INTO tags SELECT * FROM source.tags;
                INSERT OR IGNORE INTO chekis SELECT c.* FROM source.chekis c JOIN new_chekis x ON x.id=c.id;
                INSERT OR IGNORE INTO cheki_people SELECT cp.* FROM source.cheki_people cp JOIN new_chekis x ON x.id=cp.cheki_id;
                INSERT OR IGNORE INTO cheki_tags SELECT ct.* FROM source.cheki_tags ct JOIN new_chekis x ON x.id=ct.cheki_id;
                INSERT OR IGNORE INTO assets SELECT a.* FROM source.assets a JOIN restore_assets x ON x.id=a.id;
                INSERT OR IGNORE INTO cheki_assets SELECT ca.* FROM source.cheki_assets ca JOIN restore_assets x ON x.id=ca.asset_id;
                INSERT OR IGNORE INTO renditions SELECT r.* FROM source.renditions r JOIN restore_assets x ON x.id=r.asset_id;
                DROP TABLE new_chekis; DROP TABLE restore_chekis; DROP TABLE restore_assets;")?;
            if m.location_id == "local" {
                tx.execute_batch(
                    "INSERT OR IGNORE INTO person_files SELECT * FROM source.person_files;
                    INSERT OR IGNORE INTO person_documents SELECT * FROM source.person_documents;",
                )?;
            }
            // Existing asset identity is retained. Changed original bytes get the matching
            // dimensions/crop from the snapshot; previews will be regenerated as needed.
            let updated: HashSet<&str> = selected
                .iter()
                .filter(|c| c.file.role.as_deref() == Some("original"))
                .filter_map(|c| c.file.asset.as_deref())
                .collect();
            for asset in updated {
                tx.execute("UPDATE assets SET width=(SELECT width FROM source.assets WHERE id=?1),height=(SELECT height FROM source.assets WHERE id=?1),crop_json=(SELECT crop_json FROM source.assets WHERE id=?1),fingerprint=(SELECT fingerprint FROM source.assets WHERE id=?1) WHERE id=?1",[asset])?;
                tx.execute("UPDATE renditions SET width=(SELECT width FROM source.renditions WHERE asset_id=?1 AND role='original'),height=(SELECT height FROM source.renditions WHERE asset_id=?1 AND role='original'),byte_size=(SELECT byte_size FROM source.renditions WHERE asset_id=?1 AND role='original') WHERE asset_id=?1 AND role='original'",[asset])?;
            }
            tx.execute("INSERT INTO restore_runs(id) VALUES(?1)", [run])?;
            tx.commit()?;
            Ok(())
        })();
        let detach = self.db.execute_batch("DETACH DATABASE source");
        result?;
        detach?;
        // A missing or stale cache is recreated from a restored source. A cache failure leaves
        // the original and catalog intact; the preview error is visible in the library.
        for asset in selected
            .iter()
            .filter(|c| c.file.role.as_deref() == Some("original"))
            .filter_map(|c| c.file.asset.as_deref())
            .collect::<HashSet<_>>()
        {
            if let Err(e) = self.generate_cache(asset) {
                self.db.execute(
                    "UPDATE assets SET preview_error=?2 WHERE id=?1",
                    params![asset, e.to_string()],
                )?;
            }
        }
        Ok(())
    }
}
