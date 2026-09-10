use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::{Path, PathBuf}};
use uuid::Uuid;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub date: String,
    pub people: Vec<String>,
    pub event: String,
    pub tags: Vec<String>,
    pub shot_type: String,
    pub notes: String,
    pub favorite: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rendition {
    pub id: String, pub role: String, pub relative_path: String,
    pub mime_type: String, pub byte_size: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String, pub kind: String, pub src: String, pub original_path: String,
    pub filename: String, pub original_filename: String,
    pub width: Option<u32>, pub height: Option<u32>, pub byte_size: u64,
    pub preview_error: Option<String>, pub renditions: Vec<Rendition>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cheki {
    pub id: String,
    #[serde(flatten)] pub metadata: Metadata,
    pub cover_asset_id: Option<String>, pub assets: Vec<Asset>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Library { pub root: String, pub chekis: Vec<Cheki> }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport { pub imported: usize, pub errors: Vec<String>, pub library: Library }

#[derive(Serialize, Deserialize)]
struct PendingImport {
    asset: String, cheki: String, new_cheki: bool, original: String,
    original_filename: String, mime: String, size: u64, dimensions: (u32, u32),
}
pub struct Store { pub root: PathBuf, db: Connection, _lock: fs::File }
fn id() -> String { Uuid::new_v4().to_string() }
fn clean_list(values: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    values.iter().map(|v| v.trim()).filter(|v| !v.is_empty())
        .filter(|v| seen.insert(v.to_lowercase())).map(str::to_string).collect()
}
fn validate(mut m: Metadata) -> Result<Metadata> {
    if !m.date.is_empty() {
        let d = NaiveDate::parse_from_str(&m.date, "%Y-%m-%d").context("日期格式无效")?;
        if d.format("%Y-%m-%d").to_string() != m.date { bail!("日期格式应为 YYYY-MM-DD"); }
    }
    if !["solo", "2 shot", "多人切", "团切", "其他"].contains(&m.shot_type.as_str()) { bail!("无效的拍摄类型"); }
    m.people = clean_list(&m.people); m.tags = clean_list(&m.tags);
    if m.people.len() > 200 || m.tags.len() > 100 || m.people.iter().chain(m.tags.iter()).any(|s| s.chars().count() > 80) { bail!("人物或标签数量/长度超出限制"); }
    m.event = m.event.trim().to_string();
    if m.notes.len() > 50_000 || m.event.len() > 1000 { bail!("活动或备注过长"); }
    Ok(m)
}
fn filename_part(text: &str) -> String {
    let part: String = text.chars().map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c }).take(16).collect();
    part.trim_matches(|c: char| c == '.' || c.is_whitespace()).to_string()
}
pub fn asset_filename(m: &Metadata, asset_id: &str, extension: &str) -> String {
    let date = if m.date.is_empty() { "未定日期" } else { &m.date };
    let names: Vec<String> = m.people.iter().take(2).map(|n| filename_part(n)).filter(|n| !n.is_empty()).collect();
    let mut people = if names.is_empty() { "未定人物".into() } else { names.join("+") };
    if m.people.len() > 2 { people.push_str(&format!("等{}人", m.people.len())); }
    let short: String = asset_id.chars().filter(|c| *c != '-').take(10).collect();
    format!("{date}_{people}_{short}.{extension}")
}
impl Store {
    pub fn open(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;
        let root = fs::canonicalize(root)?;
        let lock = fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).open(root.join("library.lock"))?;
        lock.try_lock().context("此图库已被另一个程序实例打开")?;
        for dir in ["originals", "previews", "staging"] { fs::create_dir_all(root.join(dir))?; }
        let db = Connection::open(root.join("library.sqlite"))?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
        let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > 1 { bail!("图库版本比当前程序新，请升级程序"); }
        db.execute_batch("BEGIN;
          CREATE TABLE IF NOT EXISTS chekis(id TEXT PRIMARY KEY, date TEXT NOT NULL DEFAULT '', event TEXT NOT NULL DEFAULT '', shot_type TEXT NOT NULL DEFAULT '其他', notes TEXT NOT NULL DEFAULT '', favorite INTEGER NOT NULL DEFAULT 0, cover_asset_id TEXT);
          CREATE TABLE IF NOT EXISTS people(id TEXT PRIMARY KEY, name TEXT NOT NULL, key TEXT NOT NULL UNIQUE);
          CREATE TABLE IF NOT EXISTS tags(id TEXT PRIMARY KEY, name TEXT NOT NULL, key TEXT NOT NULL UNIQUE);
          CREATE TABLE IF NOT EXISTS cheki_people(cheki_id TEXT REFERENCES chekis(id), person_id TEXT REFERENCES people(id), position INTEGER NOT NULL, PRIMARY KEY(cheki_id,person_id));
          CREATE TABLE IF NOT EXISTS cheki_tags(cheki_id TEXT REFERENCES chekis(id), tag_id TEXT REFERENCES tags(id), position INTEGER NOT NULL, PRIMARY KEY(cheki_id,tag_id));
          CREATE TABLE IF NOT EXISTS assets(id TEXT PRIMARY KEY, kind TEXT NOT NULL DEFAULT 'unknown', original_filename TEXT NOT NULL, width INTEGER, height INTEGER, preview_error TEXT);
          CREATE TABLE IF NOT EXISTS cheki_assets(cheki_id TEXT REFERENCES chekis(id), asset_id TEXT REFERENCES assets(id), position INTEGER NOT NULL, PRIMARY KEY(cheki_id,asset_id));
          CREATE TABLE IF NOT EXISTS renditions(id TEXT PRIMARY KEY, asset_id TEXT NOT NULL REFERENCES assets(id), role TEXT NOT NULL, relative_path TEXT NOT NULL UNIQUE, mime_type TEXT NOT NULL, byte_size INTEGER NOT NULL, UNIQUE(asset_id,role));
          CREATE TABLE IF NOT EXISTS pending_renames(old_path TEXT PRIMARY KEY, new_path TEXT NOT NULL);
          CREATE INDEX IF NOT EXISTS asset_links ON cheki_assets(asset_id);
          PRAGMA user_version=1; COMMIT;")?;
        let mut store = Self {root, db, _lock: lock};
        store.recover_renames()?;
        // A durable import manifest bridges the filesystem and SQLite transaction.
        for entry in fs::read_dir(store.root.join("staging"))? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "json") {
                let pending: PendingImport = serde_json::from_reader(fs::File::open(&path)?)?;
                store.finish_import(&pending)?;
                fs::remove_file(path)?;
            }
        }
        // These copies were interrupted before their durable import manifest existed.
        for entry in fs::read_dir(store.root.join("staging"))? {
            let entry = entry?;
            if entry.file_type()?.is_file() && entry.path().extension().is_some_and(|e| e == "part") {
                fs::remove_file(entry.path())?;
            }
        }
        Ok(store)
    }
    fn exists(&self, cheki: &str) -> Result<()> {
        if !self.db.query_row("SELECT EXISTS(SELECT 1 FROM chekis WHERE id=?1)", [cheki], |r| r.get::<_, bool>(0))? { bail!("找不到收藏"); }
        Ok(())
    }
    pub fn list(&self) -> Result<Library> {
        let mut statement = self.db.prepare("SELECT id,date,event,shot_type,notes,favorite,cover_asset_id FROM chekis ORDER BY rowid DESC")?;
        let mut chekis = statement.query_map([], |r| Ok(Cheki { id:r.get(0)?, metadata: Metadata {date:r.get(1)?,event:r.get(2)?,shot_type:r.get(3)?,notes:r.get(4)?,favorite:r.get(5)?,..Metadata::default()},cover_asset_id:r.get(6)?,assets:vec![] }))?.collect::<rusqlite::Result<Vec<_>>>()?;
        for c in &mut chekis {
            c.metadata.people = self.names(&c.id, true)?; c.metadata.tags = self.names(&c.id, false)?;
            let mut stmt = self.db.prepare("SELECT a.id,a.kind,a.original_filename,a.width,a.height,a.preview_error FROM assets a JOIN cheki_assets ca ON ca.asset_id=a.id WHERE ca.cheki_id=?1 ORDER BY ca.position")?;
            c.assets = stmt.query_map([&c.id], |r| Ok(Asset {id:r.get(0)?,kind:r.get(1)?,original_filename:r.get(2)?,width:r.get(3)?,height:r.get(4)?,preview_error:r.get(5)?, src:String::new(), original_path:String::new(),filename:String::new(),byte_size:0,renditions:vec![]}))?.collect::<rusqlite::Result<Vec<_>>>()?;
            for asset in &mut c.assets {
                asset.renditions = self.db.prepare("SELECT id,role,relative_path,mime_type,byte_size FROM renditions WHERE asset_id=?1")?.query_map([&asset.id], |r| Ok(Rendition {id:r.get(0)?,role:r.get(1)?,relative_path:r.get(2)?,mime_type:r.get(3)?,byte_size:r.get::<_,i64>(4)? as u64}))?.collect::<rusqlite::Result<Vec<_>>>()?;
                if let Some(original) = asset.renditions.iter().find(|r| r.role == "original") {
                    asset.original_path = self.root.join(&original.relative_path).to_string_lossy().to_string();
                    asset.filename = Path::new(&original.relative_path).file_name().unwrap_or_default().to_string_lossy().to_string();
                    asset.byte_size = original.byte_size;
                }
                if let Some(preview) = asset.renditions.iter().find(|r| r.role == "display") { asset.src = self.root.join(&preview.relative_path).to_string_lossy().to_string(); }
            }
        }
        Ok(Library {root:self.root.to_string_lossy().to_string(), chekis})
    }
    fn names(&self, cheki: &str, people: bool) -> Result<Vec<String>> {
        let sql = if people {"SELECT p.name FROM people p JOIN cheki_people c ON p.id=c.person_id WHERE c.cheki_id=?1 ORDER BY c.position"} else {"SELECT p.name FROM tags p JOIN cheki_tags c ON p.id=c.tag_id WHERE c.cheki_id=?1 ORDER BY c.position"};
        Ok(self.db.prepare(sql)?.query_map([cheki], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?)
    }
    pub fn update(&mut self, cheki: &str, metadata: Metadata) -> Result<Library> {
        self.recover_renames()?;
        self.exists(cheki)?; let m = validate(metadata)?;
        let tx = self.db.transaction()?;
        tx.execute("UPDATE chekis SET date=?2,event=?3,shot_type=?4,notes=?5,favorite=?6 WHERE id=?1",params![cheki,m.date,m.event,m.shot_type,m.notes,m.favorite])?;
        for (table, links, column, values) in [("people","cheki_people","person_id",&m.people),("tags","cheki_tags","tag_id",&m.tags)] {
            tx.execute(&format!("DELETE FROM {links} WHERE cheki_id=?1"),[cheki])?;
            for (position, value) in values.iter().enumerate() {
                tx.execute(&format!("INSERT OR IGNORE INTO {table}(id,name,key) VALUES(?1,?2,?3)"),params![id(),value,value.to_lowercase()])?;
                let entity: String = tx.query_row(&format!("SELECT id FROM {table} WHERE key=?1"),[value.to_lowercase()],|r| r.get(0))?;
                tx.execute(&format!("INSERT INTO {links}(cheki_id,{column},position) VALUES(?1,?2,?3)"),params![cheki,entity,position as i64])?;
            }
        }
        // Commit metadata and the rename intent together. Recovery can finish a move
        // even if the process exits before the filesystem or index update completes.
        let rows: Vec<(String,String)> = tx.prepare("SELECT a.asset_id,r.relative_path FROM cheki_assets a JOIN renditions r ON r.asset_id=a.asset_id AND r.role='original' WHERE a.cheki_id=?1 AND a.rowid=(SELECT MIN(b.rowid) FROM cheki_assets b WHERE b.asset_id=a.asset_id)")?.query_map([cheki], |r| Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
        for (asset, old) in rows {
            let ext = Path::new(&old).extension().and_then(|s| s.to_str()).unwrap_or("bin");
            let new = format!("originals/{}",asset_filename(&m,&asset,ext));
            if old != new { tx.execute("INSERT OR REPLACE INTO pending_renames(old_path,new_path) VALUES(?1,?2)",params![old,new])?; }
        }
        tx.commit()?;
        self.recover_renames()?;
        self.list()
    }
    fn recover_renames(&mut self) -> Result<()> {
        let pending: Vec<(String,String)> = self.db.prepare("SELECT old_path,new_path FROM pending_renames")?.query_map([],|r| Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
        for (old,new) in pending {
            let old_file = self.root.join(&old); let new_file = self.root.join(&new);
            if old_file.exists() {
                if new_file.exists() { bail!("文件重命名冲突，原件已保留：{new}"); }
                fs::rename(&old_file,&new_file)?;
            } else if !new_file.exists() { bail!("找不到待重命名的原件：{old}"); }
            let tx = self.db.transaction()?;
            tx.execute("UPDATE renditions SET relative_path=?2 WHERE relative_path=?1",params![old,new])?;
            tx.execute("DELETE FROM pending_renames WHERE old_path=?1",[old])?; tx.commit()?;
        }
        Ok(())
    }
    pub fn import(&mut self, paths: Vec<PathBuf>, target: Option<String>) -> Result<ImportReport> {
        if let Some(ref c) = target {self.exists(c)?;}
        let mut imported = 0; let mut errors = vec![];
        for path in paths { match self.import_one(&path,target.as_deref()) { Ok(()) => imported+=1, Err(e) => errors.push(format!("{}：{e:#}",path.file_name().unwrap_or_default().to_string_lossy())) } }
        Ok(ImportReport {imported,errors,library:self.list()?})
    }
    fn import_one(&mut self, source: &Path, target: Option<&str>) -> Result<()> {
        let extension = source.extension().and_then(|s|s.to_str()).unwrap_or("").to_lowercase();
        let mime = match extension.as_str() {"jpg"|"jpeg"=>"image/jpeg","png"=>"image/png","webp"=>"image/webp","tif"|"tiff"=>"image/tiff",_=>bail!("支持 JPEG、PNG、WebP、TIFF")};
        if !source.is_file() {bail!("不是文件");}
        let source = fs::canonicalize(source)?;
        if source.starts_with(&self.root) {bail!("文件已经位于图库内，请关联已有影像");}
        let asset = id(); let cheki = target.map(str::to_string).unwrap_or_else(id);
        let m = if target.is_some() {self.list()?.chekis.into_iter().find(|c|c.id==cheki).context("收藏不存在")?.metadata} else {Metadata{shot_type:"其他".into(),..Metadata::default()}};
        let original = format!("originals/{}",asset_filename(&m,&asset,&extension));
        let staging = self.root.join(format!("staging/{asset}.part"));
        let dest = self.root.join(&original);
        if dest.exists() {bail!("文件名冲突，请重新导入");}
        let size = fs::copy(&source,&staging).context("复制原件失败")?;
        fs::File::open(&staging)?.sync_all()?;
        let dimensions = image::ImageReader::open(&staging)?.with_guessed_format()?.into_dimensions().ok();
        if dimensions.is_none() { let _=fs::remove_file(&staging); bail!("无法识别图像内容"); }
        let pending = PendingImport { asset, cheki, new_cheki: target.is_none(), original,
            original_filename: source.file_name().unwrap_or_default().to_string_lossy().to_string(),
            mime: mime.into(), size, dimensions: dimensions.unwrap() };
        let journal = self.root.join(format!("staging/{}.json",pending.asset));
        let temporary_journal = self.root.join(format!("staging/{}.journal.part",pending.asset));
        let mut file = fs::File::create(&temporary_journal)?;
        serde_json::to_writer(&mut file,&pending)?;
        file.sync_all()?;
        fs::rename(temporary_journal,&journal)?;
        // Preserve the managed original and manifest on failure for recovery at restart.
        self.finish_import(&pending)?;
        fs::remove_file(journal)?;
        Ok(())
    }
    fn finish_import(&mut self, p: &PendingImport) -> Result<()> {
        let indexed: bool = self.db.query_row("SELECT EXISTS(SELECT 1 FROM assets WHERE id=?1)",[&p.asset],|r|r.get(0))?;
        if indexed { return Ok(()); }
        let staging = self.root.join(format!("staging/{}.part",p.asset));
        let dest = self.root.join(&p.original);
        if !dest.exists() { fs::rename(staging,&dest).context("恢复导入原件失败")?; }
        let preview = format!("previews/{}.jpg",p.asset);
        let preview_path = self.root.join(&preview);
        let preview_result = make_preview(&dest,&preview_path);
        let preview_error = preview_result.as_ref().err().map(|e|format!("预览生成失败，原件已保存：{e:#}"));
        let tx = self.db.transaction()?;
        if p.new_cheki {tx.execute("INSERT INTO chekis(id,cover_asset_id) VALUES(?1,?2)",params![p.cheki,p.asset])?;}
        tx.execute("INSERT INTO assets(id,original_filename,width,height,preview_error) VALUES(?1,?2,?3,?4,?5)",params![p.asset,p.original_filename,p.dimensions.0,p.dimensions.1,preview_error])?;
        tx.execute("INSERT INTO cheki_assets(cheki_id,asset_id,position) VALUES(?1,?2,(SELECT COUNT(*) FROM cheki_assets WHERE cheki_id=?1))",params![p.cheki,p.asset])?;
        tx.execute("INSERT INTO renditions(id,asset_id,role,relative_path,mime_type,byte_size) VALUES(?1,?2,'original',?3,?4,?5)",params![id(),p.asset,p.original,p.mime,p.size as i64])?;
        if preview_result.is_ok() {tx.execute("INSERT INTO renditions(id,asset_id,role,relative_path,mime_type,byte_size) VALUES(?1,?2,'display',?3,'image/jpeg',?4)",params![id(),p.asset,preview,fs::metadata(&preview_path)?.len() as i64])?;}
        tx.commit()?;
        Ok(())
    }
    pub fn link(&mut self, cheki: &str, asset: &str) -> Result<Library> {
        self.exists(cheki)?;
        self.db.execute("INSERT OR IGNORE INTO cheki_assets(cheki_id,asset_id,position) VALUES(?1,?2,(SELECT COUNT(*) FROM cheki_assets WHERE cheki_id=?1))",params![cheki,asset])?;
        self.list()
    }
    pub fn retry_preview(&mut self, asset: &str) -> Result<Library> {
        let original: String = self.db.query_row("SELECT relative_path FROM renditions WHERE asset_id=?1 AND role='original'",[asset],|r|r.get(0))?;
        let preview = format!("previews/{asset}.jpg");
        make_preview(&self.root.join(original),&self.root.join(&preview))?;
        self.db.execute("INSERT INTO renditions(id,asset_id,role,relative_path,mime_type,byte_size) VALUES(?1,?2,'display',?3,'image/jpeg',?4) ON CONFLICT(asset_id,role) DO UPDATE SET byte_size=excluded.byte_size",params![id(),asset,preview,fs::metadata(self.root.join(&preview))?.len() as i64])?;
        self.db.execute("UPDATE assets SET preview_error=NULL WHERE id=?1",[asset])?;
        self.list()
    }
}
fn make_preview(source: &Path, destination: &Path) -> Result<()> {
    let mut reader = image::ImageReader::open(source)?.with_guessed_format()?;
    let mut limits = image::Limits::default(); limits.max_alloc = Some(512 * 1024 * 1024); reader.limits(limits);
    let mut decoder = reader.into_decoder()?;
    use image::ImageDecoder;
    let orientation = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut image = image::DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    let preview = image.thumbnail(1800,1800).to_rgb8();
    let file = fs::File::create(destination)?;
    image::codecs::jpeg::JpegEncoder::new_with_quality(&file,88).encode_image(&preview)?;
    file.sync_all()?; Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn picture(dir: &Path, name: &str) -> PathBuf {
        let path=dir.join(name);
        image::RgbImage::from_pixel(40,60,image::Rgb([120,80,100])).save(&path).unwrap(); path
    }
    #[test]
    fn import_update_reopen_preserves_original_and_relations() {
        let temp=tempfile::tempdir().unwrap(); let source=picture(temp.path(),"scan.png"); let bytes=fs::read(&source).unwrap();
        let root=temp.path().join("library");
        let id;
        {
            let mut s=Store::open(root.clone()).unwrap();
            let report=s.import(vec![source.clone()],None).unwrap(); assert_eq!(report.imported,1);
            let c=&report.library.chekis[0];id=c.id.clone();assert!(c.metadata.date.is_empty());assert_eq!(c.assets[0].width,Some(40));
            assert_eq!(c.assets[0].renditions.len(),2);
            let m=Metadata{date:"2026-08-27".into(),people:vec!["小/明".into(),"小红".into(),"小蓝".into()],event:"生日公演".into(),tags:vec!["舞台".into(),"舞台".into(),"夏日 演出".into()],shot_type:"团切".into(),notes:"备注".into(),favorite:true};
            let result=s.update(&id,m).unwrap();let c=&result.chekis[0];
            assert!(c.assets[0].filename.starts_with("2026-08-27_小_明+小红等3人_"));
            assert_eq!(c.metadata.tags.len(),2);assert_eq!(fs::read(&c.assets[0].original_path).unwrap(),bytes);
        }
        let s=Store::open(root).unwrap();let c=s.list().unwrap().chekis.remove(0);
        assert_eq!(c.id,id);assert_eq!(c.metadata.people.len(),3);assert_eq!(c.metadata.event,"生日公演");assert!(c.metadata.favorite);assert_eq!(fs::read(source).unwrap(),bytes);
    }
    #[test]
    fn related_import_and_shared_asset_do_not_duplicate_chekis_or_files() {
        let tmp=tempfile::tempdir().unwrap();let p=picture(tmp.path(),"a.png");let mut s=Store::open(tmp.path().join("lib")).unwrap();
        let first=s.import(vec![p.clone()],None).unwrap().library.chekis.remove(0);
        let attached=s.import(vec![p.clone()],Some(first.id.clone())).unwrap();assert_eq!(attached.library.chekis.len(),1);assert_eq!(attached.library.chekis[0].assets.len(),2);
        let second=s.import(vec![p],None).unwrap().library.chekis.into_iter().find(|c|c.id!=first.id).unwrap();
        s.link(&second.id,&first.assets[0].id).unwrap();s.link(&second.id,&first.assets[0].id).unwrap();
        let result=s.list().unwrap();assert_eq!(result.chekis.len(),2);assert_eq!(result.chekis.iter().find(|c|c.id==second.id).unwrap().assets.len(),2);
        assert_eq!(fs::read_dir(s.root.join("originals")).unwrap().count(),3);
    }
    #[test]
    fn recovers_rename_after_filesystem_move_before_database_update() {
        let tmp=tempfile::tempdir().unwrap();let p=picture(tmp.path(),"a.png");let root=tmp.path().join("lib");
        let old;
        {let mut s=Store::open(root.clone()).unwrap();let c=s.import(vec![p],None).unwrap().library.chekis.remove(0);old=c.assets[0].renditions.iter().find(|r|r.role=="original").unwrap().relative_path.clone();
         s.db.execute("INSERT INTO pending_renames(old_path,new_path) VALUES(?1,'originals/recovered.png')",[&old]).unwrap();fs::rename(root.join(&old),root.join("originals/recovered.png")).unwrap();}
        let s=Store::open(root).unwrap();assert_eq!(s.list().unwrap().chekis[0].assets[0].filename,"recovered.png");
    }
    #[test]
    fn malformed_files_and_invalid_metadata_are_rejected() {
        let tmp=tempfile::tempdir().unwrap();let bad=tmp.path().join("bad.png");fs::write(&bad,b"not an image").unwrap();let mut s=Store::open(tmp.path().join("lib")).unwrap();
        let result=s.import(vec![bad],None).unwrap();assert_eq!(result.imported,0);assert_eq!(result.errors.len(),1);assert!(result.library.chekis.is_empty());
        assert!(validate(Metadata{date:"2026-02-30".into(),shot_type:"solo".into(),..Metadata::default()}).is_err());
        assert!(s.import(vec![],Some("missing".into())).is_err());
    }
    #[test]
    fn imports_tiff_without_changing_the_original() {
        let tmp=tempfile::tempdir().unwrap();let p=picture(tmp.path(),"scan.tiff");let original=fs::read(&p).unwrap();let mut s=Store::open(tmp.path().join("lib")).unwrap();let r=s.import(vec![p],None).unwrap();assert_eq!(r.imported,1);let asset=&r.library.chekis[0].assets[0];assert_eq!(fs::read(&asset.original_path).unwrap(),original);assert!(asset.preview_error.is_none());assert!(!asset.src.is_empty());
    }
    #[test]
    fn recovers_import_on_either_side_of_file_move_and_only_indexes_once() {
        for moved in [false,true] {
            let tmp=tempfile::tempdir().unwrap(); let source=picture(tmp.path(),"scan.png");
            let root=tmp.path().join("lib");
            let p=PendingImport {asset:id(),cheki:id(),new_cheki:true,original:"originals/recovered.png".into(),original_filename:"scan.png".into(),mime:"image/png".into(),size:fs::metadata(&source).unwrap().len(),dimensions:(40,60)};
            let manifest;
            {
                let s=Store::open(root.clone()).unwrap();
                let staging=s.root.join(format!("staging/{}.part",p.asset));
                fs::copy(&source,&staging).unwrap();
                manifest=s.root.join(format!("staging/{}.json",p.asset));
                fs::write(&manifest,serde_json::to_vec(&p).unwrap()).unwrap();
                if moved {fs::rename(staging,s.root.join(&p.original)).unwrap();}
            }
            {let s=Store::open(root.clone()).unwrap(); assert_eq!(s.list().unwrap().chekis.len(),1);assert!(!manifest.exists());}
            // Simulate a crash after the index committed but before manifest removal.
            fs::write(&manifest,serde_json::to_vec(&p).unwrap()).unwrap();
            let s=Store::open(root).unwrap();assert_eq!(s.list().unwrap().chekis.len(),1);assert!(!manifest.exists());
        }
    }
    #[test]
    fn prevents_two_writers_for_the_same_library() {
        let tmp=tempfile::tempdir().unwrap();let first=Store::open(tmp.path().to_path_buf()).unwrap();
        assert!(Store::open(tmp.path().to_path_buf()).is_err());
        drop(first);assert!(Store::open(tmp.path().to_path_buf()).is_ok());
    }

}
