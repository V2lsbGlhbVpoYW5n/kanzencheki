use super::geometry::validate_crop;
use super::*;
use anyhow::ensure;
use image::imageops::FilterType;
use rusqlite::OptionalExtension;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Crop {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default)]
    pub quad: Option<[Point; 4]>,
    #[serde(default)]
    pub ratio: Option<f64>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub id: String,
    pub path: String,
    pub name: String,
    pub online: bool,
    pub managed: bool,
}
#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOptions {
    pub cheki_id: Option<String>,
    #[serde(default)]
    pub reference: bool,
    #[serde(default)]
    pub grouping: String,
}
pub struct ImportedFile {
    pub cheki_id: String,
    pub preview_error: Option<String>,
}
impl Store {
    pub(super) fn migrate(&mut self, version: i64) -> Result<()> {
        if version >= 2 {
            self.migrate_single_files()?;
            return Ok(());
        }
        self.db.execute_batch("BEGIN;
          ALTER TABLE chekis ADD COLUMN group_name TEXT NOT NULL DEFAULT '';
          ALTER TABLE chekis ADD COLUMN deleted_at TEXT;
          ALTER TABLE chekis ADD COLUMN cover_manual INTEGER NOT NULL DEFAULT 0;
          ALTER TABLE assets ADD COLUMN crop_json TEXT;
          ALTER TABLE assets ADD COLUMN fingerprint TEXT;
          ALTER TABLE assets ADD COLUMN preferred_source TEXT;
          ALTER TABLE renditions ADD COLUMN location_id TEXT NOT NULL DEFAULT 'local';
          ALTER TABLE renditions ADD COLUMN width INTEGER;
          ALTER TABLE renditions ADD COLUMN height INTEGER;
          CREATE TABLE renditions_v2(id TEXT PRIMARY KEY,asset_id TEXT NOT NULL REFERENCES assets(id),role TEXT NOT NULL,relative_path TEXT NOT NULL,mime_type TEXT NOT NULL,byte_size INTEGER NOT NULL,location_id TEXT NOT NULL DEFAULT 'local',width INTEGER,height INTEGER,UNIQUE(asset_id,role),UNIQUE(location_id,relative_path));
          INSERT INTO renditions_v2 SELECT * FROM renditions;
          DROP TABLE renditions;
          ALTER TABLE renditions_v2 RENAME TO renditions;
          CREATE TABLE locations(id TEXT PRIMARY KEY,path TEXT NOT NULL UNIQUE,name TEXT NOT NULL);
          CREATE TABLE pending_previews(asset_id TEXT PRIMARY KEY REFERENCES assets(id));
          UPDATE renditions SET width=(SELECT width FROM assets WHERE id=asset_id),height=(SELECT height FROM assets WHERE id=asset_id) WHERE role='original';
          UPDATE chekis SET notes=notes || char(10) || '原人物记录：' || (SELECT group_concat(p.name,'、') FROM cheki_people cp JOIN people p ON p.id=cp.person_id WHERE cp.cheki_id=chekis.id) WHERE shot_type='团切' AND EXISTS(SELECT 1 FROM cheki_people WHERE cheki_id=chekis.id);
          DELETE FROM cheki_people WHERE cheki_id IN (SELECT id FROM chekis WHERE shot_type='团切');
          PRAGMA user_version=2; COMMIT;")?;
        self.migrate_single_files()?;
        Ok(())
    }
    pub fn locations(&self) -> Result<Vec<Location>> {
        let mut result = vec![Location {
            id: "local".into(),
            path: self.root.to_string_lossy().into(),
            name: "本机图库与离线缓存".into(),
            online: true,
            managed: true,
        }];
        let mut external: Vec<Location> = self
            .db
            .prepare("SELECT id,path,name FROM locations")?
            .query_map([], |r| {
                Ok(Location {
                    id: r.get(0)?,
                    path: r.get(1)?,
                    name: r.get(2)?,
                    online: false,
                    managed: true,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        for l in &mut external {
            l.online = Path::new(&l.path).is_dir();
        }
        result.extend(external);
        Ok(result)
    }
    pub fn add_location(&mut self, path: &Path, replace: Option<&str>) -> Result<Library> {
        let path = fs::canonicalize(path)?;
        if !path.is_dir() {
            bail!("请选择文件夹");
        }
        if path.starts_with(&self.root) || self.root.starts_with(&path) {
            bail!("外部目录不能包含本机图库或位于本机图库中");
        }
        if let Some(location) = replace {
            if location == "local" {
                bail!("本机缓存目录暂不支持迁移");
            }
            let paths: Vec<String> = self
                .db
                .prepare("SELECT relative_path FROM renditions WHERE location_id=?1")?
                .query_map([location], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            if paths.iter().any(|p| !path.join(p).is_file()) {
                bail!("新目录缺少已登记原件，请选择原文件夹的完整副本");
            }
            if self.db.execute(
                "UPDATE locations SET path=?2 WHERE id=?1",
                params![location, path.to_string_lossy()],
            )? == 0
            {
                bail!("找不到目录");
            }
        } else {
            self.db.execute(
                "INSERT OR IGNORE INTO locations(id,path,name) VALUES(?1,?2,?3)",
                params![
                    id(),
                    path.to_string_lossy(),
                    path.file_name().unwrap_or_default().to_string_lossy()
                ],
            )?;
        }
        self.list()
    }
    pub(super) fn path_for(&self, r: &Rendition) -> Result<PathBuf> {
        let root = if r.location_id == "local" {
            self.root.clone()
        } else {
            PathBuf::from(self.db.query_row(
                "SELECT path FROM locations WHERE id=?1",
                [&r.location_id],
                |r| r.get::<_, String>(0),
            )?)
        };
        Ok(root.join(&r.relative_path))
    }
    pub(super) fn hydrate_asset(&self, a: &mut Asset) -> Result<()> {
        for r in &mut a.renditions {
            r.available = self.path_for(r)?.is_file();
        }
        let best = a
            .renditions
            .iter()
            .filter(|r| is_source(&r.role))
            .max_by_key(|r| quality(r));
        if let Some(r) = best {
            a.original_path = self.path_for(r)?.to_string_lossy().into();
            a.filename = Path::new(&r.relative_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into();
            a.byte_size = r.byte_size;
            a.width = r.width.or(a.width);
            a.height = r.height.or(a.height);
        }
        a.src = a
            .renditions
            .iter()
            .find(|r| r.role == "display" && r.available)
            .map(|r| self.path_for(r).unwrap().to_string_lossy().into())
            .unwrap_or_default();
        a.base_src = a
            .renditions
            .iter()
            .find(|r| r.role == "base" && r.available)
            .map(|r| self.path_for(r).unwrap().to_string_lossy().into())
            .unwrap_or_else(|| a.src.clone());
        Ok(())
    }
    pub(super) fn choose_cover(&self, c: &mut Cheki) {
        if !c.cover_manual
            || !c
                .assets
                .iter()
                .any(|a| Some(&a.id) == c.cover_asset_id.as_ref())
        {
            c.cover_asset_id = c
                .assets
                .iter()
                .max_by_key(|a| {
                    let q = a
                        .renditions
                        .iter()
                        .filter(|r| is_source(&r.role))
                        .map(quality)
                        .max()
                        .unwrap_or(0);
                    (!a.src.is_empty(), q)
                })
                .map(|a| a.id.clone());
        }
    }
    pub fn trash(&mut self, ids: &[String], restore: bool) -> Result<Library> {
        let tx = self.db.transaction()?;
        for c in ids {
            tx.execute(
                "UPDATE chekis SET deleted_at=?2 WHERE id=?1",
                params![
                    c,
                    if restore {
                        None
                    } else {
                        Some(chrono::Utc::now().to_rfc3339())
                    }
                ],
            )?;
        }
        tx.commit()?;
        self.list()
    }
    pub fn cover(&mut self, cheki: &str, asset: Option<String>) -> Result<Library> {
        if let Some(ref a) = asset {
            if !self.db.query_row(
                "SELECT EXISTS(SELECT 1 FROM cheki_assets WHERE cheki_id=?1 AND asset_id=?2)",
                params![cheki, a],
                |r| r.get::<_, bool>(0),
            )? {
                bail!("影像不属于这张收藏");
            }
        }
        self.db.execute(
            "UPDATE chekis SET cover_asset_id=?2,cover_manual=?3 WHERE id=?1",
            params![cheki, asset, asset.is_some()],
        )?;
        self.list()
    }
    pub fn import_file(&mut self, source: &Path, options: &ImportOptions) -> Result<ImportedFile> {
        if let Some(ref c) = options.cheki_id {
            self.exists(c)?;
        }
        if !options.reference {
            self.import_one(source, options.cheki_id.as_deref())?;
            let asset: String = self.db.query_row(
                "SELECT id FROM assets ORDER BY rowid DESC LIMIT 1",
                [],
                |r| r.get(0),
            )?;
            self.db.execute("UPDATE renditions SET width=(SELECT width FROM assets WHERE id=?1),height=(SELECT height FROM assets WHERE id=?1) WHERE asset_id=?1 AND role='original'",[&asset])?;
            // The original is committed. Cache failures must not turn this into a failed import.
            let preview_result = (|| -> Result<()> {
                let display: String = self.db.query_row(
                    "SELECT relative_path FROM renditions WHERE asset_id=?1 AND role='display'",
                    [&asset],
                    |r| r.get(0),
                )?;
                let base = format!("previews/{asset}-base-{}.jpg", id());
                fs::copy(self.root.join(display), self.root.join(&base))?;
                self.generated(&asset, "base", &base)?;
                self.render_crop(&asset, None)
            })();
            if let Err(e) = preview_result {
                self.db.execute(
                    "UPDATE assets SET preview_error=?2 WHERE id=?1",
                    params![asset, format!("{e:#}")],
                )?;
            }
            let (cheki_id, _) = self.latest_link()?;
            return Ok(ImportedFile {
                cheki_id,
                preview_error: self.asset_preview_error(&asset)?,
            });
        }
        let source = fs::canonicalize(source)?;
        let extension = source
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let mime = match extension.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "tif" | "tiff" => "image/tiff",
            "webp" => "image/webp",
            _ => bail!("不支持的图像格式"),
        };
        let dims = image::ImageReader::open(&source)?
            .with_guessed_format()?
            .into_dimensions()?;
        let asset = id();
        let cheki = options.cheki_id.clone().unwrap_or_else(id);
        let loc = self
            .locations()?
            .into_iter()
            .filter(|l| l.id != "local" && source.starts_with(&l.path))
            .max_by_key(|l| l.path.len())
            .context("先在设置中添加原件所在目录")?;
        let relative = source
            .strip_prefix(&loc.path)?
            .to_string_lossy()
            .to_string();
        let location = loc.id;
        let duplicate: bool = self.db.query_row(
            "SELECT EXISTS(SELECT 1 FROM renditions WHERE location_id=?1 AND relative_path=?2)",
            params![location, relative],
            |r| r.get(0),
        )?;
        if duplicate {
            bail!("文件已登记，请在相册中归并收藏");
        }
        let tx = self.db.transaction()?;
        {
            if options.cheki_id.is_none() {
                tx.execute(
                    "INSERT INTO chekis(id,cover_asset_id) VALUES(?1,?2)",
                    params![cheki, asset],
                )?;
            }
            tx.execute(
                "INSERT INTO assets(id,kind,original_filename,width,height) VALUES(?1,?2,?3,?4,?5)",
                params![
                    asset,
                    "unknown",
                    source.file_name().unwrap().to_string_lossy(),
                    dims.0,
                    dims.1
                ],
            )?;
            tx.execute("INSERT INTO cheki_assets(cheki_id,asset_id,position) VALUES(?1,?2,(SELECT COUNT(*) FROM cheki_assets WHERE cheki_id=?1))",params![cheki,asset])?;
        }
        let role = "original";
        tx.execute("INSERT INTO renditions(id,asset_id,role,relative_path,mime_type,byte_size,location_id,width,height) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![id(),asset,role,relative,mime,fs::metadata(&source)?.len() as i64,location,dims.0,dims.1])?;
        tx.commit()?;
        let metadata = self
            .list()?
            .chekis
            .into_iter()
            .find(|c| c.id == cheki)
            .context("收藏不存在")?
            .metadata;
        self.update(&cheki, metadata)?;
        let _ = self.refresh_preview(&asset);
        Ok(ImportedFile {
            cheki_id: cheki,
            preview_error: self.asset_preview_error(&asset)?,
        })
    }
    pub fn asset_preview_error(&self, asset: &str) -> Result<Option<String>> {
        Ok(self.db.query_row(
            "SELECT preview_error FROM assets WHERE id=?1",
            [asset],
            |r| r.get(0),
        )?)
    }
    pub fn latest_link(&self) -> Result<(String, String)> {
        Ok(self.db.query_row(
            "SELECT cheki_id,asset_id FROM cheki_assets ORDER BY rowid DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    }
    pub fn location_files(&self, location: &str) -> Result<Vec<PathBuf>> {
        let loc = self
            .locations()?
            .into_iter()
            .find(|l| l.id == location && l.id != "local")
            .context("请选择外部目录")?;
        if !loc.online {
            bail!("目录离线");
        }
        let mut files = vec![];
        let mut dirs = vec![PathBuf::from(loc.path)];
        while let Some(dir) = dirs.pop() {
            for item in fs::read_dir(dir)? {
                let item = item?;
                if item.file_name().to_string_lossy().starts_with(".cheki-") {
                    continue;
                }
                let ty = item.file_type()?;
                if ty.is_symlink() {
                    continue;
                }
                if ty.is_dir() {
                    dirs.push(item.path());
                } else if ty.is_file() {
                    let path = item.path();
                    let ext = path
                        .extension()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase();
                    if ["jpg", "jpeg", "png", "webp", "tif", "tiff"].contains(&ext.as_str()) {
                        files.push(path);
                    }
                }
            }
        }
        let indexed: HashSet<PathBuf> = self
            .db
            .prepare("SELECT relative_path FROM renditions WHERE location_id=?1")?
            .query_map([location], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .map(|p| {
                PathBuf::from(
                    &self
                        .locations()
                        .unwrap()
                        .into_iter()
                        .find(|l| l.id == location)
                        .unwrap()
                        .path,
                )
                .join(p)
            })
            .collect();
        files.retain(|p| !indexed.contains(p));
        files.sort();
        Ok(files)
    }
    pub fn refresh_preview(&mut self, asset: &str) -> Result<()> {
        let result = self.generate_cache(asset);
        if let Err(ref e) = result {
            self.db.execute(
                "UPDATE assets SET preview_error=?2 WHERE id=?1",
                params![asset, format!("{e:#}")],
            )?;
        }
        result
    }
    pub(super) fn generated(&self, asset: &str, role: &str, path: &str) -> Result<()> {
        let old: Option<String> = self
            .db
            .query_row(
                "SELECT relative_path FROM renditions WHERE asset_id=?1 AND role=?2",
                params![asset, role],
                |r| r.get(0),
            )
            .optional()?;
        let d = image::image_dimensions(self.root.join(path))?;
        self.db.execute("INSERT INTO renditions(id,asset_id,role,relative_path,mime_type,byte_size,width,height) VALUES(?1,?2,?3,?4,'image/jpeg',?5,?6,?7) ON CONFLICT(asset_id,role) DO UPDATE SET relative_path=excluded.relative_path,byte_size=excluded.byte_size,width=excluded.width,height=excluded.height",params![id(),asset,role,path,fs::metadata(self.root.join(path))?.len() as i64,d.0,d.1])?;
        if let Some(old) = old {
            if old != path && old.starts_with("previews/") {
                let _ = fs::remove_file(self.root.join(old));
            }
        }
        Ok(())
    }
    pub(super) fn generate_cache(&mut self, asset: &str) -> Result<()> {
        let a = self
            .list()?
            .chekis
            .into_iter()
            .flat_map(|c| c.assets)
            .find(|a| a.id == asset)
            .context("找不到影像")?;
        let source = a
            .renditions
            .iter()
            .filter(|r| is_source(&r.role) && r.available)
            .max_by_key(|r| quality(r))
            .context("原件离线，操作失败，请连接目录后重新操作")?;
        let base = format!("previews/{asset}-base-{}.jpg", id());
        make_preview(&self.path_for(source)?, &self.root.join(&base))?;
        self.generated(asset, "base", &base)?;
        self.render_crop(asset, a.crop.as_ref())?;

        self.db
            .execute("UPDATE assets SET preview_error=NULL WHERE id=?1", [asset])?;
        Ok(())
    }
    pub(super) fn render_crop(&mut self, asset: &str, crop: Option<&Crop>) -> Result<()> {
        let base: String = self
            .db
            .query_row(
                "SELECT relative_path FROM renditions WHERE asset_id=?1 AND role='base'",
                [asset],
                |r| r.get(0),
            )
            .optional()?
            .or(self
                .db
                .query_row(
                    "SELECT relative_path FROM renditions WHERE asset_id=?1 AND role='display'",
                    [asset],
                    |r| r.get(0),
                )
                .optional()?)
            .context("暂无本机预览，连接原件后生成")?;
        let image = image::open(self.root.join(base))?;
        let output = super::geometry::apply_crop(image, crop)?;
        let path = format!("previews/{asset}-display-{}.jpg", id());
        output.to_rgb8().save(self.root.join(&path))?;
        self.generated(asset, "display", &path)?;
        let small = output.resize_exact(9, 8, FilterType::Triangle).to_luma8();
        let mut hash = 0u64;
        for y in 0..8 {
            for x in 0..8 {
                hash = (hash << 1)
                    | u64::from(small.get_pixel(x, y)[0] > small.get_pixel(x + 1, y)[0]);
            }
        }
        self.db.execute(
            "UPDATE assets SET fingerprint=?2 WHERE id=?1",
            params![asset, format!("{hash:016x}")],
        )?;
        Ok(())
    }
    pub fn crop(&mut self, asset: &str, crop: Option<Crop>) -> Result<Library> {
        if let Some(ref c) = crop {
            super::geometry::validate_crop(c)?;
        }
        // Reuse the unedited local cache: cropping works even while the disk is offline.
        let has_base: bool = self.db.query_row(
            "SELECT EXISTS(SELECT 1 FROM renditions WHERE asset_id=?1 AND role='base')",
            [asset],
            |r| r.get(0),
        )?;
        if !has_base {
            bail!("本机浏览图不可用，无法裁切");
        }
        self.render_crop(asset, crop.as_ref())?;
        self.db.execute(
            "UPDATE assets SET crop_json=?2 WHERE id=?1",
            params![asset, crop.as_ref().map(serde_json::to_string).transpose()?],
        )?;
        self.list()
    }
    pub fn auto_crop(&self, asset: &str) -> Result<Crop> {
        let a = self
            .list()?
            .chekis
            .into_iter()
            .flat_map(|c| c.assets)
            .find(|a| a.id == asset)
            .context("找不到影像")?;
        let img = image::open(a.base_src)?.to_rgb8();
        suggest_cheki_crop(&img)
    }
}
/// Detect the dominant card against a reasonably uniform scanner or tabletop background.
/// Returns a proposal only; the editor lets the user inspect and adjust all four corners.
fn suggest_cheki_crop(source: &image::RgbImage) -> Result<Crop> {
    use std::collections::VecDeque;
    let (sw, sh) = source.dimensions();
    ensure!(sw >= 48 && sh >= 48, "图像太小，无法判断拍立得边界");
    let scale = (700.0 / sw.max(sh) as f64).min(1.0);
    let w = ((sw as f64 * scale).round() as u32).max(1);
    let h = ((sh as f64 * scale).round() as u32).max(1);
    let img = image::imageops::resize(source, w, h, FilterType::Triangle);
    let mut samples = [Vec::new(), Vec::new(), Vec::new()];
    for x in (0..w).step_by(4) {
        for y in [0, 1, h - 2, h - 1] {
            for c in 0..3 {
                samples[c].push(img.get_pixel(x, y).0[c]);
            }
        }
    }
    for y in (0..h).step_by(4) {
        for x in [0, 1, w - 2, w - 1] {
            for c in 0..3 {
                samples[c].push(img.get_pixel(x, y).0[c]);
            }
        }
    }
    let mut bg = [0u8; 3];
    let mut deviations = Vec::new();
    for c in 0..3 {
        samples[c].sort_unstable();
        bg[c] = samples[c][samples[c].len() / 2];
    }
    for i in 0..samples[0].len() {
        deviations.push(
            (0..3)
                .map(|c| (samples[c][i] as i16 - bg[c] as i16).unsigned_abs() as u32)
                .sum::<u32>(),
        );
    }
    deviations.sort_unstable();
    let variation = deviations[deviations.len() / 2];
    ensure!(variation < 65, "背景较复杂，暂无法可靠判断边界，请手动裁切");
    let threshold = 30u32.max(variation * 3);
    let size = (w * h) as usize;
    let mut mask = vec![false; size];
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y).0;
            let delta = (0..3)
                .map(|c| (p[c] as i16 - bg[c] as i16).unsigned_abs() as u32)
                .sum::<u32>();
            mask[(y * w + x) as usize] = delta > threshold;
        }
    }
    // A small closing step joins artwork and white margins into one card region.
    for _ in 0..2 {
        let before = mask.clone();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = (y * w + x) as usize;
                if !before[i]
                    && [i - 1, i + 1, i - w as usize, i + w as usize]
                        .into_iter()
                        .filter(|&n| before[n])
                        .count()
                        >= 3
                {
                    mask[i] = true;
                }
            }
        }
    }
    let mut seen = vec![false; size];
    let mut best = Vec::new();
    for start in 0..size {
        if !mask[start] || seen[start] {
            continue;
        }
        let mut q = VecDeque::from([start]);
        seen[start] = true;
        let mut part = Vec::new();
        while let Some(i) = q.pop_front() {
            part.push(i);
            let x = i % w as usize;
            let y = i / w as usize;
            for neighbor in [
                if x > 0 { Some(i - 1) } else { None },
                if x + 1 < w as usize {
                    Some(i + 1)
                } else {
                    None
                },
                if y > 0 { Some(i - w as usize) } else { None },
                if y + 1 < h as usize {
                    Some(i + w as usize)
                } else {
                    None
                },
            ] {
                if let Some(n) = neighbor {
                    if mask[n] && !seen[n] {
                        seen[n] = true;
                        q.push_back(n);
                    }
                }
            }
        }
        if part.len() > best.len() {
            best = part;
        }
    }
    ensure!(
        (best.len() as f64) > (size as f64) * 0.07 && (best.len() as f64) < (size as f64) * 0.88,
        "未检测到清晰的拍立得边界，请手动裁切"
    );
    let mut x0 = w;
    let mut y0 = h;
    let mut x1 = 0;
    let mut y1 = 0;
    let (mut tl, mut tr, mut br, mut bl) = (None, None, None, None);
    for i in best {
        let x = (i % w as usize) as u32;
        let y = (i / w as usize) as u32;
        x0 = x0.min(x);
        x1 = x1.max(x);
        y0 = y0.min(y);
        y1 = y1.max(y);
        let sum = x + y;
        let diff = x as i64 - y as i64;
        if tl.is_none_or(|(_, v)| sum < v) {
            tl = Some(((x, y), sum));
        }
        if br.is_none_or(|(_, v)| sum > v) {
            br = Some(((x, y), sum));
        }
        if tr.is_none_or(|(_, v)| diff > v) {
            tr = Some(((x, y), diff));
        }
        if bl.is_none_or(|(_, v)| diff < v) {
            bl = Some(((x, y), diff));
        }
    }
    let bw = x1.saturating_sub(x0);
    let bh = y1.saturating_sub(y0);
    ensure!(
        bw > 20 && bh > 20 && x0 > 1 && y0 > 1 && x1 < w - 2 && y1 < h - 2,
        "未检测到完整的拍立得边界，请手动裁切"
    );
    let aspect = bw as f64 / bh as f64;
    ensure!(
        (0.48..=2.05).contains(&aspect),
        "检测到的区域比例不像拍立得，请手动裁切"
    );
    let margin = 2.0;
    let mut result = Crop {
        x: (x0 as f64 - margin).max(0.0) / w as f64,
        y: (y0 as f64 - margin).max(0.0) / h as f64,
        w: (bw as f64 + 2.0 * margin).min(w as f64) / w as f64,
        h: (bh as f64 + 2.0 * margin).min(h as f64) / h as f64,
        ..Default::default()
    };
    let points = [tl.unwrap().0, tr.unwrap().0, br.unwrap().0, bl.unwrap().0].map(|(x, y)| Point {
        x: x as f64 / w as f64,
        y: y as f64 / h as f64,
    });
    let axis_error = (((points[0].y - points[1].y).abs() + (points[2].y - points[3].y).abs())
        * h as f64
        / bh as f64)
        + (((points[0].x - points[3].x).abs() + (points[1].x - points[2].x).abs()) * w as f64
            / bw as f64);
    if axis_error > 0.09 {
        result.quad = Some(points);
    }
    validate_crop(&result)?;
    Ok(result)
}

fn is_source(role: &str) -> bool {
    role == "original"
}
fn quality(r: &Rendition) -> u64 {
    (r.width.unwrap_or(0) as u64 * r.height.unwrap_or(0) as u64) * 4
        + if r.mime_type == "image/tiff" { 2 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn suggests_card_bounds_and_perspective_corners() {
        let mut flat = image::RgbImage::from_pixel(400, 500, image::Rgb([34, 39, 44]));
        for y in 65..425 {
            for x in 90..315 {
                flat.put_pixel(x, y, image::Rgb([228, 224, 208]));
            }
        }
        let region = suggest_cheki_crop(&flat).unwrap();
        assert!((region.x - 0.225).abs() < 0.02);
        assert!((region.y - 0.13).abs() < 0.02);
        assert!(region.quad.is_none());

        let mut tilted = image::RgbImage::from_pixel(400, 500, image::Rgb([35, 40, 44]));
        let corners = [(93.0, 72.0), (306.0, 94.0), (282.0, 433.0), (72.0, 404.0)];
        for y in 0..500 {
            for x in 0..400 {
                let inside = (0..4).all(|i| {
                    let a = corners[i];
                    let b = corners[(i + 1) % 4];
                    (b.0 - a.0) * (y as f64 - a.1) - (b.1 - a.1) * (x as f64 - a.0) >= 0.0
                });
                if inside {
                    tilted.put_pixel(x, y, image::Rgb([232, 226, 210]));
                }
            }
        }
        let region = suggest_cheki_crop(&tilted).unwrap();
        assert!(region.quad.is_some());
        let q = region.quad.unwrap();
        assert!((q[0].x - 93.0 / 400.0).abs() < 0.04);
        assert!((q[2].y - 433.0 / 500.0).abs() < 0.04);
    }
    fn photo(root: &Path, name: &str, w: u32, h: u32) -> PathBuf {
        let p = root.join(name);
        image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 255) as u8, (y % 255) as u8, 100])
        })
        .save(&p)
        .unwrap();
        p
    }
    #[test]
    fn external_originals_are_renamed_and_cached_crop_survives_disconnect() {
        let tmp = tempfile::tempdir().unwrap();
        let disk = tmp.path().join("disk");
        fs::create_dir(&disk).unwrap();
        let p = photo(&disk, "scan.tiff", 240, 360);
        let bytes = fs::read(&p).unwrap();
        let root = tmp.path().join("local");
        let mut s = Store::open(root.clone()).unwrap();
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
        let aid = c.assets[0].id.clone();
        assert!(Path::new(&c.assets[0].base_src).is_file());
        let mut m = c.metadata;
        m.date = "2026-08-27".into();
        m.shot_type = "团切".into();
        m.group = "测试团体".into();
        m.people = vec!["不应关联".into()];
        s.update(&c.id, m).unwrap();
        let renamed = PathBuf::from(s.list().unwrap().chekis[0].assets[0].original_path.clone());
        assert_ne!(p, renamed);
        assert_eq!(fs::read(&renamed).unwrap(), bytes);
        fs::rename(&disk, tmp.path().join("unplugged")).unwrap();
        let crop = Crop {
            x: 0.1,
            y: 0.1,
            w: 0.6,
            h: 0.8,
            ..Default::default()
        };
        s.crop(&aid, Some(crop)).unwrap();
        assert!(s.refresh_preview(&aid).is_err());
        assert!(!s
            .db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='pending_previews')",
                [],
                |r| r.get::<_, bool>(0)
            )
            .unwrap());
        drop(s);
        let mut s = Store::open(root).unwrap();
        let c = s.list().unwrap().chekis.remove(0);
        assert_eq!(c.metadata.group, "测试团体");
        assert!(c.metadata.people.is_empty());
        assert!(
            !c.assets[0]
                .renditions
                .iter()
                .find(|r| r.role == "original")
                .unwrap()
                .available
        );
        assert!(Path::new(&c.assets[0].src).is_file());
        assert!(c.assets[0].crop.is_some());
        fs::rename(tmp.path().join("unplugged"), &disk).unwrap();
        s.refresh_preview(&aid).unwrap();
        assert_eq!(fs::read(renamed).unwrap(), bytes);
    }
    #[test]
    fn registered_reference_with_failed_preview_can_group_and_rebuild() {
        let tmp = tempfile::tempdir().unwrap();
        let disk = tmp.path().join("disk");
        fs::create_dir(&disk).unwrap();
        let first = photo(&disk, "first.png", 40, 60);
        let second = photo(&disk, "second.png", 40, 60);
        let mut s = Store::open(tmp.path().join("library")).unwrap();
        s.add_location(&disk, None).unwrap();
        fs::remove_dir(s.root.join("previews")).unwrap();
        fs::write(s.root.join("previews"), b"blocked").unwrap();
        let options = ImportOptions {
            reference: true,
            ..Default::default()
        };
        let registered = s.import_file(&first, &options).unwrap();
        assert!(registered.preview_error.is_some());
        assert_eq!(s.list().unwrap().chekis.len(), 1);
        let asset_id = s.list().unwrap().chekis[0].assets[0].id.clone();
        fs::remove_file(s.root.join("previews")).unwrap();
        fs::create_dir(s.root.join("previews")).unwrap();
        s.import_file(
            &second,
            &ImportOptions {
                cheki_id: Some(registered.cheki_id.clone()),
                reference: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(s.list().unwrap().chekis[0].assets.len(), 2);
        s.refresh_preview(&asset_id).unwrap();
        let restored = s.list().unwrap();
        let asset = restored.chekis[0]
            .assets
            .iter()
            .find(|a| a.id == asset_id)
            .unwrap();
        assert!(asset.preview_error.is_none());
        assert!(Path::new(&asset.src).is_file());
    }
    #[test]
    fn same_relative_names_in_different_roots_and_repeat_scan_are_safe() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Store::open(tmp.path().join("local")).unwrap();
        for name in ["disk1", "disk2"] {
            let disk = tmp.path().join(name);
            fs::create_dir(&disk).unwrap();
            let p = photo(&disk, "same.png", 40, 60);
            s.add_location(&disk, None).unwrap();
            s.import_file(
                &p,
                &ImportOptions {
                    reference: true,
                    ..Default::default()
                },
            )
            .unwrap();
        }
        assert_eq!(s.list().unwrap().chekis.len(), 2);
        for l in s
            .locations()
            .unwrap()
            .into_iter()
            .filter(|l| l.id != "local")
        {
            assert!(s.location_files(&l.id).unwrap().is_empty());
        }
    }
    #[test]
    fn independent_files_choose_best_cover() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Store::open(tmp.path().join("lib")).unwrap();
        let low = photo(tmp.path(), "low.jpg", 40, 60);
        s.import_file(&low, &Default::default()).unwrap();
        let c = s.list().unwrap().chekis.remove(0);
        let high = photo(tmp.path(), "high.tiff", 200, 300);
        s.import_file(
            &high,
            &ImportOptions {
                cheki_id: Some(c.id.clone()),
                ..Default::default()
            },
        )
        .unwrap();
        let c = s.list().unwrap().chekis.remove(0);
        assert_eq!(c.assets.len(), 2);
        assert!(c.assets.iter().all(|a| a
            .renditions
            .iter()
            .filter(|r| is_source(&r.role))
            .count()
            == 1));
        assert_eq!(c.cover_asset_id, Some(c.assets[1].id.clone()));
        assert_eq!(
            s.cover(&c.id, Some(c.assets[0].id.clone())).unwrap().chekis[0].cover_asset_id,
            Some(c.assets[0].id.clone())
        );
    }
    #[test]
    fn trash_restore_and_merge_keep_sources_and_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let p = photo(tmp.path(), "source.png", 40, 60);
        let mut s = Store::open(tmp.path().join("lib")).unwrap();
        s.import_file(&p, &Default::default()).unwrap();
        s.import_file(&p, &Default::default()).unwrap();
        let cs = s.list().unwrap().chekis;
        let a = cs[0].id.clone();
        let b = cs[1].id.clone();
        s.merge_many(&a, &[b.clone()]).unwrap();
        let cs = s.list().unwrap().chekis;
        assert_eq!(cs.iter().find(|c| c.id == a).unwrap().assets.len(), 2);
        assert!(cs.iter().find(|c| c.id == b).unwrap().deleted_at.is_some());
        s.trash(&[a.clone(), b.clone()], false).unwrap();
        s.trash(&[a, b], true).unwrap();
        for c in s.list().unwrap().chekis {
            assert!(c.deleted_at.is_none());
            for a in c.assets {
                assert!(Path::new(&a.original_path).is_file());
            }
        }
    }
    #[test]
    fn tiny_crop_at_image_edge_remains_a_valid_pixel() {
        let tmp = tempfile::tempdir().unwrap();
        let p = photo(tmp.path(), "edge.png", 40, 60);
        let mut s = Store::open(tmp.path().join("lib")).unwrap();
        s.import_file(&p, &Default::default()).unwrap();
        let c = s.list().unwrap().chekis.remove(0);
        let result = s
            .crop(
                &c.assets[0].id,
                Some(Crop {
                    x: 0.99,
                    y: 0.99,
                    w: 0.01,
                    h: 0.01,
                    ..Default::default()
                }),
            )
            .unwrap();
        assert_eq!(
            image::image_dimensions(&result.chekis[0].assets[0].src).unwrap(),
            (1, 1)
        );
    }
    #[test]
    fn crop_validation_and_restoring_full_frame_are_lossless() {
        let tmp = tempfile::tempdir().unwrap();
        let p = photo(tmp.path(), "source.png", 100, 150);
        let mut s = Store::open(tmp.path().join("lib")).unwrap();
        s.import_file(&p, &Default::default()).unwrap();
        let c = s.list().unwrap().chekis.remove(0);
        let aid = &c.assets[0].id;
        assert!(s
            .crop(
                aid,
                Some(Crop {
                    x: 0.8,
                    y: 0.0,
                    w: 0.5,
                    h: 1.0,
                    ..Default::default()
                })
            )
            .is_err());
        s.crop(
            aid,
            Some(Crop {
                x: 0.1,
                y: 0.1,
                w: 0.5,
                h: 0.5,
                ..Default::default()
            }),
        )
        .unwrap();
        let result = s.crop(aid, None).unwrap();
        assert_eq!(
            image::image_dimensions(&result.chekis[0].assets[0].src).unwrap(),
            (100, 150)
        );
    }
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    #[test]
    fn opens_existing_v1_library_and_preserves_group_people_as_notes() {
        let tmp = tempfile::tempdir().unwrap();
        {
            let db = Connection::open(tmp.path().join("library.sqlite")).unwrap();
            db.execute_batch(include_str!("schema-v1.sql")).unwrap();
            db.execute("INSERT INTO chekis(id,shot_type,notes,date) VALUES('c','团切','原备注','2026-08-27')",[]).unwrap();
            db.execute(
                "INSERT INTO people(id,name,key) VALUES('p','原人物','原人物')",
                [],
            )
            .unwrap();
            db.execute("INSERT INTO cheki_people VALUES('c','p',0)", [])
                .unwrap();
        }
        {
            let s = Store::open(tmp.path().to_path_buf()).unwrap();
            let c = s.list().unwrap().chekis.remove(0);
            assert!(c.metadata.people.is_empty());
            assert!(c.metadata.notes.contains("原备注"));
            assert!(c.metadata.notes.contains("原人物记录：原人物"));
            assert_eq!(c.metadata.date, "2026-08-27");
            assert!(c.metadata.group.is_empty());
        }
        let s = Store::open(tmp.path().to_path_buf()).unwrap();
        assert_eq!(
            s.list().unwrap().chekis[0]
                .metadata
                .notes
                .matches("原人物记录")
                .count(),
            1
        );
        assert_eq!(
            s.db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            5
        );
    }
}

#[cfg(test)]
mod large_image_test {
    use super::*;
    #[test]
    #[ignore = "Opt-in: generates and imports a 1 GB uncompressed TIFF"]
    fn gigabyte_tiff_generates_offline_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("large.tiff");
        let created = std::process::Command::new("magick")
            .args([
                "-limit",
                "memory",
                "256MiB",
                "-limit",
                "map",
                "512MiB",
                "-limit",
                "disk",
                "8GiB",
                "-limit",
                "thread",
                "2",
                "-size",
                "19000x19000",
                "xc:#ece8e0",
                "-type",
                "TrueColor",
                "-depth",
                "8",
                "-compress",
                "None",
            ])
            .env("MAGICK_TEMPORARY_PATH", tmp.path())
            .arg(&source)
            .output()
            .unwrap();
        assert!(
            created.status.success(),
            "{}",
            String::from_utf8_lossy(&created.stderr)
        );
        let size = fs::metadata(&source).unwrap().len();
        assert!(size > 1_000_000_000, "fixture should exceed one GB: {size}");
        let mut store = Store::open(tmp.path().join("local")).unwrap();
        store
            .import_file(
                &source,
                &ImportOptions {
                    ..Default::default()
                },
            )
            .unwrap();
        let c = store.list().unwrap().chekis.remove(0);
        let asset = &c.assets[0];
        assert_eq!(asset.byte_size, size);
        assert_eq!(asset.width, Some(19000));
        assert!(asset.preview_error.is_none(), "{:?}", asset.preview_error);
        assert_eq!(
            image::image_dimensions(&asset.base_src).unwrap(),
            (1800, 1800)
        );
        eprintln!("Imported {size} byte TIFF; original and 1800 × 1800 offline cache verified");
    }
}
