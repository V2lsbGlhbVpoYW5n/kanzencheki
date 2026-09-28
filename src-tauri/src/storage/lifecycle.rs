use super::*;
impl Store {
    pub(super) fn migrate_single_files(&mut self) -> Result<()> {
        let version: i64 = self.db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version >= 3 {
            return Ok(());
        }
        // Preserve every old source as an independent asset, including offline files.
        let versions: Vec<(String, String, String)> = self
            .db
            .prepare(
                "SELECT id,asset_id,relative_path FROM renditions WHERE role LIKE 'version:%'",
            )?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let tx = self.db.transaction()?;
        for (rid, old, path) in versions {
            let aid = id();
            tx.execute("INSERT INTO assets(id,original_filename,width,height,crop_json,fingerprint) SELECT ?1,?2,r.width,r.height,a.crop_json,a.fingerprint FROM renditions r JOIN assets a ON a.id=r.asset_id WHERE r.id=?3",params![aid,Path::new(&path).file_name().unwrap_or_default().to_string_lossy(),rid])?;
            tx.execute("INSERT INTO cheki_assets(cheki_id,asset_id,position) SELECT cheki_id,?1,position+1 FROM cheki_assets WHERE asset_id=?2",params![aid,old])?;
            tx.execute(
                "UPDATE renditions SET asset_id=?2,role='original' WHERE id=?1",
                params![rid, aid],
            )?;
            let caches:Vec<(String,String)>=tx.prepare("SELECT role,relative_path FROM renditions WHERE asset_id=?1 AND role IN ('base','display')")?.query_map([&old],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
            for (role, source) in caches {
                if self.root.join(&source).is_file() {
                    let dest = format!("previews/{aid}-{role}.jpg");
                    fs::copy(self.root.join(&source), self.root.join(&dest))?;
                    tx.execute("INSERT INTO renditions(id,asset_id,role,relative_path,mime_type,byte_size,width,height) SELECT ?1,?2,role,?3,mime_type,byte_size,width,height FROM renditions WHERE asset_id=?4 AND role=?5",params![id(),aid,dest,old,role])?;
                }
            }
        }
        tx.execute_batch("DROP TABLE pending_previews; UPDATE assets SET preferred_source=NULL,kind='unknown'; ALTER TABLE pending_renames RENAME TO pending_renames_old; CREATE TABLE pending_renames(old_path TEXT NOT NULL,new_path TEXT NOT NULL,location_id TEXT NOT NULL DEFAULT 'local',PRIMARY KEY(location_id,old_path)); INSERT INTO pending_renames(old_path,new_path) SELECT old_path,new_path FROM pending_renames_old; DROP TABLE pending_renames_old; PRAGMA user_version=3;")?;
        tx.commit()?;
        Ok(())
    }
    pub fn merge_many(&mut self, target: &str, sources: &[String]) -> Result<Library> {
        if sources.is_empty() || sources.iter().any(|s| s == target) {
            bail!("请选择至少两张不同收藏");
        }
        let tx = self.db.transaction()?;
        for c in std::iter::once(target).chain(sources.iter().map(String::as_str)) {
            let active: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM chekis WHERE id=?1 AND deleted_at IS NULL)",
                [c],
                |r| r.get(0),
            )?;
            if !active {
                bail!("只能归并相册中的收藏");
            }
        }
        for source in sources {
            tx.execute("INSERT OR IGNORE INTO cheki_assets(cheki_id,asset_id,position) SELECT ?1,asset_id,position+(SELECT COUNT(*) FROM cheki_assets WHERE cheki_id=?1) FROM cheki_assets WHERE cheki_id=?2",params![target,source])?;
            tx.execute(
                "UPDATE chekis SET deleted_at=?2 WHERE id=?1",
                params![source, chrono::Utc::now().to_rfc3339()],
            )?;
        }
        tx.commit()?;
        self.list()
    }
    pub fn purge(&mut self, ids: &[String]) -> Result<Library> {
        self.purge_with(ids, |p| trash::delete(p).map_err(Into::into))
    }
    fn purge_with(
        &mut self,
        ids: &[String],
        trash_file: impl FnMut(&Path) -> Result<()>,
    ) -> Result<Library> {
        let lib = self.list()?;
        for id in ids {
            if !lib
                .chekis
                .iter()
                .any(|c| &c.id == id && c.deleted_at.is_some())
            {
                bail!("只能永久删除回收站中的收藏");
            }
        }
        let assets: Vec<Asset> = lib
            .chekis
            .iter()
            .filter(|c| ids.contains(&c.id))
            .flat_map(|c| c.assets.clone())
            .filter(|a| {
                !lib.chekis
                    .iter()
                    .any(|c| !ids.contains(&c.id) && c.assets.iter().any(|other| other.id == a.id))
            })
            .collect();
        self.erase_assets(&assets, true, trash_file)?;
        let tx = self.db.transaction()?;
        for id in ids {
            for table in ["cheki_assets", "cheki_people", "cheki_tags"] {
                tx.execute(&format!("DELETE FROM {table} WHERE cheki_id=?1"), [id])?;
            }
            tx.execute("DELETE FROM chekis WHERE id=?1", [id])?;
        }
        tx.commit()?;
        self.list()
    }
    pub fn delete_asset(&mut self, cheki: &str, asset: &str) -> Result<Library> {
        let lib = self.list()?;
        let c = lib
            .chekis
            .iter()
            .find(|c| c.id == cheki)
            .context("收藏不存在")?;
        let a = c
            .assets
            .iter()
            .find(|a| a.id == asset)
            .context("影像不属于收藏")?;
        // Shared records may exist after a recoverable merge. Only remove this reference.
        if lib
            .chekis
            .iter()
            .any(|c| c.id != cheki && c.assets.iter().any(|a| a.id == asset))
        {
            self.db.execute(
                "DELETE FROM cheki_assets WHERE cheki_id=?1 AND asset_id=?2",
                params![cheki, asset],
            )?;
        } else {
            self.erase_assets(&[a.clone()], true, |p| trash::delete(p).map_err(Into::into))?;
        }
        self.remove_empty_chekis()?;
        self.list()
    }
    pub fn remove_location(&mut self, location: &str) -> Result<Library> {
        if location == "local" {
            bail!("本机仓库不能通过移除外置目录操作删除");
        }
        let exists: bool = self.db.query_row(
            "SELECT EXISTS(SELECT 1 FROM locations WHERE id=?1)",
            [location],
            |r| r.get(0),
        )?;
        if !exists {
            bail!("目录不存在");
        }
        let assets: Vec<Asset> = self
            .list()?
            .chekis
            .into_iter()
            .flat_map(|c| c.assets)
            .filter(|a| {
                a.renditions
                    .iter()
                    .any(|r| r.role == "original" && r.location_id == location)
            })
            .collect();
        // Unregistering a location never touches its originals, even when online.
        self.erase_assets(&assets, false, |_| bail!("移除目录不得操作原件"))?;
        self.remove_empty_chekis()?;
        let tx = self.db.transaction()?;
        tx.execute(
            "DELETE FROM pending_renames WHERE location_id=?1",
            [location],
        )?;
        tx.execute("DELETE FROM locations WHERE id=?1", [location])?;
        tx.commit()?;
        self.list()
    }
    pub fn clear_local(&mut self, confirmation: &str) -> Result<Library> {
        if confirmation != "清空本机仓库" {
            bail!("请输入完整确认文字：清空本机仓库");
        }
        self.recover_document_writes()?;
        let assets: Vec<Asset> = self
            .list()?
            .chekis
            .into_iter()
            .flat_map(|c| c.assets)
            .filter(|a| {
                a.renditions
                    .iter()
                    .any(|r| r.role == "original" && r.location_id == "local")
            })
            .collect();
        self.erase_assets(&assets, true, |p| trash::delete(p).map_err(Into::into))?;
        self.remove_empty_chekis()?;
        let people_dir = self.root.join("people");
        if fs::read_dir(&people_dir)?.next().is_some() {
            trash::delete(&people_dir).context("无法将人物附件和文章移入系统回收站")?;
            fs::create_dir_all(&people_dir)?;
        }
        let tx = self.db.transaction()?;
        tx.execute("DELETE FROM person_files", [])?;
        tx.execute("DELETE FROM person_documents", [])?;
        tx.execute("DELETE FROM document_writes", [])?;
        tx.commit()?;
        self.list()
    }
    fn remove_empty_chekis(&self) -> Result<()> {
        self.db.execute_batch("BEGIN; DELETE FROM cheki_people WHERE cheki_id IN (SELECT id FROM chekis WHERE NOT EXISTS(SELECT 1 FROM cheki_assets WHERE cheki_id=chekis.id)); DELETE FROM cheki_tags WHERE cheki_id IN (SELECT id FROM chekis WHERE NOT EXISTS(SELECT 1 FROM cheki_assets WHERE cheki_id=chekis.id)); DELETE FROM chekis WHERE NOT EXISTS(SELECT 1 FROM cheki_assets WHERE cheki_id=chekis.id); COMMIT;")?;
        Ok(())
    }
    fn erase_assets(
        &mut self,
        assets: &[Asset],
        remove_originals: bool,
        mut trash_file: impl FnMut(&Path) -> Result<()>,
    ) -> Result<()> {
        let mut seen = HashSet::new();
        let assets: Vec<_> = assets
            .iter()
            .filter(|a| seen.insert(a.id.clone()))
            .collect();
        // Preflight the entire request. A disconnected disk must not create a queued job.
        for a in &assets {
            let r = a
                .renditions
                .iter()
                .find(|r| r.role == "original")
                .context("影像缺少原件记录")?;
            for cache in a.renditions.iter().filter(|r| r.role != "original") {
                if cache.location_id != "local"
                    || !cache.relative_path.starts_with("previews/")
                    || Path::new(&cache.relative_path)
                        .components()
                        .any(|c| matches!(c, std::path::Component::ParentDir))
                {
                    bail!("缓存路径不合法，停止清理");
                }
            }
            if remove_originals && !self.path_for(r)?.is_file() {
                bail!("原件离线，操作失败：{}", a.filename);
            }
        }
        let mut errors = vec![];
        for a in assets {
            let r = a.renditions.iter().find(|r| r.role == "original").unwrap();
            if remove_originals {
                trash_file(&self.path_for(r)?)
                    .with_context(|| format!("无法移入系统回收站：{}", a.filename))?;
            }
            for r in &a.renditions {
                if r.role == "original" {
                    continue;
                }
                if r.location_id != "local" || !r.relative_path.starts_with("previews/") {
                    bail!("缓存路径不合法，停止清理");
                }
                let p = self.path_for(r)?;
                if let Err(e) = fs::remove_file(&p) {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        errors.push(format!("缓存清理失败 {}：{e}", p.display()));
                    }
                }
            }
            let tx = self.db.transaction()?;
            tx.execute(
                "DELETE FROM pending_renames WHERE location_id=?1 AND (old_path=?2 OR new_path=?2)",
                params![r.location_id, r.relative_path],
            )?;
            tx.execute(
                "UPDATE chekis SET cover_asset_id=NULL,cover_manual=0 WHERE cover_asset_id=?1",
                [&a.id],
            )?;
            tx.execute("DELETE FROM cheki_assets WHERE asset_id=?1", [&a.id])?;
            tx.execute("DELETE FROM renditions WHERE asset_id=?1", [&a.id])?;
            tx.execute("DELETE FROM assets WHERE id=?1", [&a.id])?;
            tx.commit()?;
        }
        if !errors.is_empty() {
            bail!("{}", errors.join("；"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(root: &Path) -> PathBuf {
        let p = root.join("source.png");
        image::RgbImage::new(40, 60).save(&p).unwrap();
        p
    }
    #[test]
    fn purge_preserves_shared_original_and_removes_all_unreferenced_caches() {
        let tmp = tempfile::tempdir().unwrap();
        let p = fixture(tmp.path());
        let mut s = Store::open(tmp.path().join("library")).unwrap();
        s.import_file(&p, &Default::default()).unwrap();
        s.import_file(&p, &Default::default()).unwrap();
        let cs = s.list().unwrap().chekis;
        let target = &cs[0].id;
        let source = &cs[1].id;
        s.merge_many(target, &[source.clone()]).unwrap();
        s.purge_with(&[source.clone()], |_| {
            bail!("shared originals must never be trashed")
        })
        .unwrap();
        assert_eq!(s.list().unwrap().chekis[0].assets.len(), 2);
        s.trash(&[target.clone()], false).unwrap();
        let mut count = 0;
        s.purge_with(&[target.clone()], |p| {
            count += 1;
            fs::rename(p, tmp.path().join(format!("trashed-{count}.png")))?;
            Ok(())
        })
        .unwrap();
        assert_eq!(count, 2);
        assert!(s.list().unwrap().chekis.is_empty());
        assert_eq!(fs::read_dir(s.root.join("previews")).unwrap().count(), 0);
        assert_eq!(
            s.db.query_row("SELECT COUNT(*) FROM renditions", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn offline_delete_and_rename_fail_without_pending_jobs_or_metadata_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let disk = tmp.path().join("disk");
        fs::create_dir(&disk).unwrap();
        let p = fixture(&disk);
        let mut s = Store::open(tmp.path().join("library")).unwrap();
        s.add_location(&disk, None).unwrap();
        s.import_file(
            &p,
            &ImportOptions {
                reference: true,
                ..Default::default()
            },
        )
        .unwrap();
        let c = s.list().unwrap().chekis.remove(0);
        fs::rename(&disk, tmp.path().join("unplugged")).unwrap();
        let mut m = c.metadata.clone();
        m.date = "2026-09-11".into();
        assert!(s.update(&c.id, m).is_err());
        assert_eq!(s.list().unwrap().chekis[0].metadata.date, c.metadata.date);
        assert!(s.delete_asset(&c.id, &c.assets[0].id).is_err());
        assert_eq!(s.list().unwrap().chekis[0].assets.len(), 1);
        assert_eq!(
            s.db.query_row("SELECT COUNT(*) FROM pending_renames", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn v2_versions_migrate_without_losing_originals_or_offline_previews() {
        let tmp = tempfile::tempdir().unwrap();
        let p = fixture(tmp.path());
        let root = tmp.path().join("library");
        let mut s = Store::open(root.clone()).unwrap();
        s.import_file(&p, &Default::default()).unwrap();
        let a = s.list().unwrap().chekis[0].assets[0].id.clone();
        fs::copy(&p, root.join("originals/old-version.png")).unwrap();
        s.db.execute("INSERT INTO renditions(id,asset_id,role,relative_path,mime_type,byte_size,width,height) VALUES('legacy',?1,'version:legacy','originals/old-version.png','image/png',100,40,60)",[&a]).unwrap();
        s.db.execute_batch("ALTER TABLE pending_renames RENAME TO rename_v3; CREATE TABLE pending_renames(old_path TEXT PRIMARY KEY,new_path TEXT NOT NULL); DROP TABLE rename_v3; CREATE TABLE pending_previews(asset_id TEXT PRIMARY KEY REFERENCES assets(id)); ALTER TABLE people DROP COLUMN description; ALTER TABLE people DROP COLUMN aliases_json; ALTER TABLE people DROP COLUMN notes; ALTER TABLE people DROP COLUMN deleted_at; ALTER TABLE people DROP COLUMN created_at; DROP TABLE person_files; DROP TABLE person_documents; DROP TABLE document_writes; PRAGMA user_version=2;").unwrap();
        drop(s);
        let s = Store::open(root).unwrap();
        let c = s.list().unwrap().chekis.remove(0);
        assert_eq!(c.assets.len(), 2);
        for a in c.assets {
            assert!(Path::new(&a.original_path).is_file());
            assert!(Path::new(&a.src).is_file());
            assert_eq!(
                a.renditions.iter().filter(|r| r.role == "original").count(),
                1
            );
        }
    }
    #[test]
    fn removing_external_location_preserves_originals_online_and_offline() {
        for offline in [false, true] {
            let tmp = tempfile::tempdir().unwrap();
            let disk = tmp.path().join("disk");
            fs::create_dir(&disk).unwrap();
            let p = fixture(&disk);
            let bytes = fs::read(&p).unwrap();
            let local = fixture(tmp.path());
            let mut s = Store::open(tmp.path().join("library")).unwrap();
            s.add_location(&disk, None).unwrap();
            let location = s
                .locations()
                .unwrap()
                .into_iter()
                .find(|l| l.id != "local")
                .unwrap()
                .id;
            s.import_file(&local, &Default::default()).unwrap();
            let c = s.list().unwrap().chekis.remove(0);
            s.import_file(
                &p,
                &ImportOptions {
                    reference: true,
                    cheki_id: Some(c.id.clone()),
                    ..Default::default()
                },
            )
            .unwrap();
            let before = s.list().unwrap().chekis.remove(0);
            let ext = before
                .assets
                .iter()
                .find(|a| a.renditions.iter().any(|r| r.location_id == location))
                .unwrap();
            let filename = Path::new(&ext.original_path)
                .file_name()
                .unwrap()
                .to_owned();
            let cache = ext.src.clone();
            s.trash(&[c.id.clone()], false).unwrap();
            if offline {
                fs::rename(&disk, tmp.path().join("unplugged")).unwrap();
            }
            assert!(s.remove_location("local").is_err());
            s.remove_location(&location).unwrap();
            let root = if offline {
                tmp.path().join("unplugged")
            } else {
                disk
            };
            assert_eq!(fs::read(root.join(filename)).unwrap(), bytes);
            assert!(!Path::new(&cache).exists());
            let after = s.list().unwrap();
            assert_eq!(after.locations.len(), 1);
            assert_eq!(after.chekis.len(), 1);
            assert_eq!(after.chekis[0].assets.len(), 1);
            assert!(after.chekis[0].deleted_at.is_some());
            assert!(Path::new(&after.chekis[0].assets[0].original_path).is_file());
            assert!(Path::new(&after.chekis[0].assets[0].src).is_file());
        }
    }
    #[test]
    fn real_system_trash_and_local_clear_are_isolated() {
        // Child process isolates XDG variables from parallel tests and the user's trash.
        if let Some(data) = std::env::var_os("CHEKI_TRASH_TEST_DATA") {
            let data = PathBuf::from(data);
            let p = fixture(&data);
            let disk = data.join("external");
            fs::create_dir(&disk).unwrap();
            let external = fixture(&disk);
            let mut s = Store::open(data.join("library")).unwrap();
            s.add_location(&disk, None).unwrap();
            s.import_file(&p, &Default::default()).unwrap();
            let c = s.list().unwrap().chekis.remove(0);
            s.import_file(
                &external,
                &ImportOptions {
                    reference: true,
                    cheki_id: Some(c.id.clone()),
                    ..Default::default()
                },
            )
            .unwrap();
            let person = s
                .save_person(PersonDraft {
                    name: "测试人物".into(),
                    ..Default::default()
                })
                .unwrap();
            let attachment = data.join("attachment.txt");
            fs::write(&attachment, "attachment").unwrap();
            s.import_person_file(&person.id, &attachment).unwrap();
            s.save_document(DocumentDraft {
                id: None,
                person_id: person.id.clone(),
                title: "article".into(),
                body: "body".into(),
                revision: None,
            })
            .unwrap();
            let backup = data.join("before-clear.tar.lz4");
            let snapshot = s
                .backup_archive("local", &backup, None, |_, _, _| {})
                .unwrap()
                .snapshot;
            assert!(s.clear_local("wrong phrase").is_err());
            let before = s.list().unwrap().chekis.remove(0);
            let ext = before
                .assets
                .iter()
                .find(|a| {
                    a.renditions
                        .iter()
                        .any(|r| r.role == "original" && r.location_id != "local")
                })
                .unwrap();
            let extpath = ext.original_path.clone();
            let cache = ext.src.clone();
            s.clear_local("清空本机仓库").unwrap();
            assert_eq!(s.person_space(&person.id).unwrap().files.len(), 0);
            assert_eq!(s.person_space(&person.id).unwrap().documents.len(), 0);
            assert!(s.people().unwrap().iter().any(|p| p.id == person.id));
            assert_eq!(fs::read_dir(s.root.join("people")).unwrap().count(), 0);
            assert_eq!(s.list().unwrap().chekis[0].assets.len(), 1);
            assert!(Path::new(&extpath).is_file());
            assert!(Path::new(&cache).is_file());
            let a = s.list().unwrap().chekis[0].assets[0].clone();
            s.delete_asset(&c.id, &a.id).unwrap();
            assert!(s.list().unwrap().chekis.is_empty());
            assert!(!Path::new(&extpath).exists());
            assert_eq!(fs::read_dir(s.root.join("previews")).unwrap().count(), 0);
            assert_eq!(fs::read_dir(data.join("Trash/files")).unwrap().count(), 3);
            assert_eq!(fs::read_dir(data.join("Trash/info")).unwrap().count(), 3);
            let plan = s.restore_plan(&backup, &snapshot.id, None).unwrap();
            assert_eq!(plan.unchanged, 0);
            assert_eq!(plan.missing, snapshot.file_count);
            let mut progress_labels = Vec::new();
            let result = s
                .restore_backup(
                    &backup,
                    &snapshot.id,
                    None,
                    super::backup::RestorePolicy::Skip,
                    &plan.token,
                    |_, _, label| progress_labels.push(label.to_string()),
                )
                .unwrap();
            assert_eq!(result.restored, snapshot.file_count);
            assert!(!progress_labels
                .iter()
                .any(|label| label == "正在生成浏览图…"));
            assert_eq!(fs::read_dir(s.root.join("previews")).unwrap().count(), 2);
            assert_eq!(s.person_space(&person.id).unwrap().files.len(), 1);
            assert_eq!(s.person_space(&person.id).unwrap().documents.len(), 1);
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "storage::lifecycle::tests::real_system_trash_and_local_clear_are_isolated",
                "--nocapture",
            ])
            .env("XDG_DATA_HOME", tmp.path())
            .env("CHEKI_TRASH_TEST_DATA", tmp.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
