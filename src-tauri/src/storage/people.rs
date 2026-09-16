use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: String,
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub notes: String,
    pub deleted_at: Option<String>,
    pub created_at: String,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonDraft {
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub allow_duplicate: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonFile {
    pub id: String,
    pub person_id: String,
    pub filename: String,
    pub original_filename: String,
    pub src: String,
    pub mime_type: String,
    pub byte_size: u64,
    pub created_at: String,
    pub available: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonDocument {
    pub id: String,
    pub person_id: String,
    pub title: String,
    pub filename: String,
    pub updated_at: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonSpace {
    pub files: Vec<PersonFile>,
    pub documents: Vec<PersonDocument>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentContent {
    pub body: String,
    pub revision: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDraft {
    pub id: Option<String>,
    pub person_id: String,
    pub title: String,
    pub body: String,
    pub revision: Option<String>,
}
impl Store {
    pub(super) fn migrate_people(&mut self) -> Result<()> {
        let version: i64 = self.db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version >= 4 {
            return Ok(());
        }
        self.db.execute_batch("BEGIN;
          ALTER TABLE people ADD COLUMN description TEXT NOT NULL DEFAULT '';
          ALTER TABLE people ADD COLUMN aliases_json TEXT NOT NULL DEFAULT '[]';
          ALTER TABLE people ADD COLUMN notes TEXT NOT NULL DEFAULT '';
          ALTER TABLE people ADD COLUMN deleted_at TEXT;
          ALTER TABLE people ADD COLUMN created_at TEXT NOT NULL DEFAULT '';
          CREATE TABLE person_files(id TEXT PRIMARY KEY,person_id TEXT NOT NULL REFERENCES people(id),filename TEXT NOT NULL,original_filename TEXT NOT NULL,mime_type TEXT NOT NULL,byte_size INTEGER NOT NULL,created_at TEXT NOT NULL);
          CREATE TABLE person_documents(id TEXT PRIMARY KEY,person_id TEXT NOT NULL REFERENCES people(id),title TEXT NOT NULL,filename TEXT NOT NULL,updated_at TEXT NOT NULL);
          CREATE TABLE document_writes(document_id TEXT PRIMARY KEY,person_id TEXT NOT NULL,title TEXT NOT NULL,filename TEXT NOT NULL,body TEXT NOT NULL,updated_at TEXT NOT NULL);
          PRAGMA user_version=4; COMMIT;")?;
        Ok(())
    }
    pub fn people(&self) -> Result<Vec<Person>> {
        Ok(self.db.prepare("SELECT id,name,description,aliases_json,notes,deleted_at,created_at FROM people ORDER BY name,id")?.query_map([],|r|Ok(Person{id:r.get(0)?,name:r.get(1)?,description:r.get(2)?,aliases:serde_json::from_str(&r.get::<_,String>(3)?).unwrap_or_default(),notes:r.get(4)?,deleted_at:r.get(5)?,created_at:r.get(6)?}))?.collect::<rusqlite::Result<_>>()?)
    }
    fn person(&self, person: &str, active: bool) -> Result<Person> {
        let p = self
            .people()?
            .into_iter()
            .find(|p| p.id == person)
            .context("人物不存在")?;
        if active && p.deleted_at.is_some() {
            bail!("请先从人物回收站恢复此人物");
        }
        Ok(p)
    }
    fn person_dir(&self, person: &str) -> Result<PathBuf> {
        // Existing v1 IDs may not be UUIDs; still prohibit path traversal.
        if person.is_empty()
            || !person
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            bail!("无效人物 ID");
        }
        Ok(self.root.join("people").join(person))
    }
    pub fn save_person(&mut self, mut d: PersonDraft) -> Result<Person> {
        d.name = d.name.trim().into();
        d.description = d.description.trim().into();
        if d.name.is_empty()
            || d.name.chars().count() > 80
            || d.description.chars().count() > 60
            || d.notes.len() > 50000
        {
            bail!("请填写名字（最多80字）和简短区分说明（最多60字）");
        }
        let people = self.people()?;
        let old = if let Some(ref id) = d.id {
            Some(self.person(id, true)?)
        } else {
            None
        };
        let same: Vec<_> = people
            .iter()
            .filter(|p| {
                Some(&p.id) != d.id.as_ref() && p.name.to_lowercase() == d.name.to_lowercase()
            })
            .collect();
        if !same.is_empty() {
            if d.description.is_empty() {
                bail!("创建或改为重名人物时，请填写简短区分说明，例如所属团体");
            }
            if old.is_none() && !d.allow_duplicate {
                bail!("已有人物使用这个名字，请选择“创建同名人物”");
            }
            if same
                .iter()
                .any(|p| p.description.to_lowercase() == d.description.to_lowercase())
            {
                bail!("同名人物已有相同的区分说明，请使用不同说明");
            }
        }
        if let Some(ref p) = old {
            if p.name != d.name {
                d.aliases.push(p.name.clone());
            }
        }
        let aliases: Vec<_> = clean_list(&d.aliases)
            .into_iter()
            .filter(|a| a.to_lowercase() != d.name.to_lowercase())
            .collect();
        if aliases.len() > 50 || aliases.iter().any(|a| a.chars().count() > 80) {
            bail!("别名过多或过长");
        }
        let pid = d.id.unwrap_or_else(id);
        let created = old
            .map(|p| p.created_at)
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
        let dir = self.person_dir(&pid)?;
        fs::create_dir_all(dir.join("originals"))?;
        fs::create_dir_all(dir.join("documents"))?;
        self.db.execute("INSERT INTO people(id,name,key,description,aliases_json,notes,created_at) VALUES(?1,?2,?1,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET name=excluded.name,description=excluded.description,aliases_json=excluded.aliases_json,notes=excluded.notes",params![pid,d.name,d.description,serde_json::to_string(&aliases)?,d.notes,created])?;
        self.person(&pid, false)
    }
    pub(super) fn resolve_people(&mut self, m: &mut Metadata) -> Result<Vec<String>> {
        if m.shot_type == "团切" {
            m.people.clear();
            m.people_ids = Some(vec![]);
            return Ok(vec![]);
        }
        let ids = if let Some(ref ids) = m.people_ids {
            clean_list(ids)
        } else {
            let mut ids = vec![];
            for name in &m.people {
                let found: Vec<_> = self
                    .people()?
                    .into_iter()
                    .filter(|p| {
                        p.deleted_at.is_none() && p.name.to_lowercase() == name.to_lowercase()
                    })
                    .collect();
                if found.len() > 1 {
                    bail!("人物“{name}”有重名，请明确选择人物");
                }
                ids.push(if let Some(p) = found.first() {
                    p.id.clone()
                } else {
                    self.save_person(PersonDraft {
                        name: name.clone(),
                        ..Default::default()
                    })?
                    .id
                });
            }
            ids
        };
        if ids.len() > 200 {
            bail!("关联人物数量超过上限");
        }
        m.people = ids
            .iter()
            .map(|p| self.person(p, false).map(|p| p.name))
            .collect::<Result<_>>()?;
        m.people_ids = Some(ids.clone());
        Ok(ids)
    }
    pub fn person_trash(&mut self, person: &str, restore: bool) -> Result<Library> {
        self.person(person, false)?;
        self.db.execute(
            "UPDATE people SET deleted_at=?2 WHERE id=?1",
            params![
                person,
                if restore {
                    None
                } else {
                    Some(chrono::Utc::now().to_rfc3339())
                }
            ],
        )?;
        self.list()
    }
    pub fn person_purge(&mut self, person: &str) -> Result<Library> {
        let p = self.person(person, false)?;
        if p.deleted_at.is_none() {
            bail!("请先将人物移入回收站");
        }
        self.recover_document_writes()?;
        let dir = self.person_dir(person)?;
        if dir.exists() {
            trash::delete(&dir).context("无法将人物附件和文章移入系统回收站")?;
        }
        let tx = self.db.transaction()?;
        tx.execute("DELETE FROM cheki_people WHERE person_id=?1", [person])?;
        tx.execute("DELETE FROM person_files WHERE person_id=?1", [person])?;
        tx.execute("DELETE FROM person_documents WHERE person_id=?1", [person])?;
        tx.execute("DELETE FROM people WHERE id=?1", [person])?;
        tx.commit()?;
        self.list()
    }
    pub fn person_space(&self, person: &str) -> Result<PersonSpace> {
        self.person(person, false)?;
        let dir = self.person_dir(person)?;
        let files=self.db.prepare("SELECT id,filename,original_filename,mime_type,byte_size,created_at FROM person_files WHERE person_id=?1 ORDER BY rowid DESC")?.query_map([person],|r|{
            let filename:String=r.get(1)?;let path=dir.join("originals").join(&filename);
            Ok(PersonFile{id:r.get(0)?,person_id:person.into(),filename,original_filename:r.get(2)?,src:path.to_string_lossy().into(),mime_type:r.get(3)?,byte_size:r.get::<_,i64>(4)? as u64,created_at:r.get(5)?,available:path.is_file()})
        })?.collect::<rusqlite::Result<_>>()?;
        let documents=self.db.prepare("SELECT id,title,filename,updated_at FROM person_documents WHERE person_id=?1 ORDER BY updated_at DESC,id")?.query_map([person],|r|Ok(PersonDocument{id:r.get(0)?,person_id:person.into(),title:r.get(1)?,filename:r.get(2)?,updated_at:r.get(3)?}))?.collect::<rusqlite::Result<_>>()?;
        Ok(PersonSpace { files, documents })
    }
    pub fn import_person_file(&mut self, person: &str, source: &Path) -> Result<()> {
        self.person(person, true)?;
        let source = fs::canonicalize(source)?;
        if !source.is_file() {
            bail!("请选择文件");
        }
        if source.starts_with(&self.root) {
            bail!("文件已在图库中，请引用已有附件");
        }
        let fid = id();
        let original = source
            .file_name()
            .context("缺少文件名")?
            .to_string_lossy()
            .to_string();
        let ext = source.extension().and_then(|s| s.to_str()).unwrap_or("");
        let filename = format!(
            "{}_{}{}",
            filename_part(
                source
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .as_ref()
            ),
            &fid[..10],
            if ext.is_empty() {
                String::new()
            } else {
                format!(".{}", filename_part(ext))
            }
        );
        let dir = self.person_dir(person)?.join("originals");
        fs::create_dir_all(&dir)?;
        let temp = dir.join(format!("{fid}.part"));
        let dest = dir.join(&filename);
        fs::copy(&source, &temp)?;
        fs::File::open(&temp)?.sync_all()?;
        fs::rename(&temp, &dest)?;
        let result=self.db.execute("INSERT INTO person_files(id,person_id,filename,original_filename,mime_type,byte_size,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![fid,person,filename,original,mime_guess::from_path(&source).first_or_octet_stream().to_string(),fs::metadata(&dest)?.len() as i64,chrono::Utc::now().to_rfc3339()]);
        if let Err(e) = result {
            let _ = fs::remove_file(&dest);
            return Err(e.into());
        }
        Ok(())
    }
    pub fn rename_person_file(
        &mut self,
        person: &str,
        file: &str,
        name: &str,
    ) -> Result<PersonSpace> {
        self.person(person, true)?;
        let f = self
            .person_space(person)?
            .files
            .into_iter()
            .find(|f| f.id == file)
            .context("附件不存在")?;
        let name = name.trim();
        if name.is_empty()
            || name.len() > 240
            || name == "."
            || name == ".."
            || name.ends_with(['.', ' '])
            || name
                .chars()
                .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        {
            bail!("请输入有效文件名，不要包含路径或特殊字符");
        }
        if Path::new(name).extension() != Path::new(&f.filename).extension() {
            bail!("请保留原文件扩展名");
        }
        let source = PathBuf::from(&f.src);
        if !source.is_file() {
            bail!("附件文件不可用，无法改名");
        }
        let dest = source.with_file_name(name);
        if source == dest {
            return self.person_space(person);
        }
        // Hard-link creation is exclusive: an existing destination is never overwritten.
        fs::hard_link(&source, &dest).context("无法改名，目标文件名可能已存在")?;
        if let Err(e) = self.db.execute(
            "UPDATE person_files SET filename=?1 WHERE id=?2",
            params![name, file],
        ) {
            let _ = fs::remove_file(&dest);
            return Err(e.into());
        }
        if let Err(e) = fs::remove_file(&source) {
            self.db.execute(
                "UPDATE person_files SET filename=?1 WHERE id=?2",
                params![f.filename, file],
            )?;
            let _ = fs::remove_file(&dest);
            return Err(e.into());
        }
        self.person_space(person)
    }
    pub fn delete_person_file(&mut self, person: &str, file: &str) -> Result<PersonSpace> {
        self.person(person, true)?;
        let f = self
            .person_space(person)?
            .files
            .into_iter()
            .find(|f| f.id == file)
            .context("附件不存在")?;
        trash::delete(Path::new(&f.src)).context("无法将附件移入系统回收站")?;
        self.db
            .execute("DELETE FROM person_files WHERE id=?1", [file])?;
        self.person_space(person)
    }
    pub fn read_document(&self, person: &str, document: &str) -> Result<DocumentContent> {
        let doc = self
            .person_space(person)?
            .documents
            .into_iter()
            .find(|d| d.id == document)
            .context("文章不存在")?;
        let body = fs::read_to_string(
            self.person_dir(person)?
                .join("documents")
                .join(doc.filename),
        )?;
        Ok(DocumentContent {
            revision: body.clone(),
            body,
        })
    }
    pub fn save_document(&mut self, d: DocumentDraft) -> Result<PersonSpace> {
        self.person(&d.person_id, true)?;
        self.recover_document_writes()?;
        let title = d.title.trim();
        if title.is_empty() || title.chars().count() > 150 || d.body.len() > 2_000_000 {
            bail!("请填写标题（最多150字）；文章最多2MB");
        }
        let did = d.id.clone().unwrap_or_else(id);
        let filename = if d.id.is_some() {
            let existing = self.read_document(&d.person_id, &did)?;
            if d.revision.as_deref() != Some(&existing.revision) {
                bail!("文章已被其他编辑修改，请重新打开后合并内容");
            }
            self.person_space(&d.person_id)?
                .documents
                .into_iter()
                .find(|x| x.id == did)
                .unwrap()
                .filename
        } else {
            format!("{}_{}.md", filename_part(title), &did[..10])
        };
        // A write journal bridges atomic file replacement and the SQLite index.
        self.db.execute("INSERT INTO document_writes(document_id,person_id,title,filename,body,updated_at) VALUES(?1,?2,?3,?4,?5,?6)",params![did,d.person_id,title,filename,d.body,chrono::Utc::now().to_rfc3339()])?;
        self.recover_document_writes()?;
        self.person_space(&d.person_id)
    }
    pub(super) fn recover_document_writes(&mut self) -> Result<()> {
        let writes: Vec<(String, String, String, String, String, String)> = self
            .db
            .prepare(
                "SELECT document_id,person_id,title,filename,body,updated_at FROM document_writes",
            )?
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        for (id, person, title, filename, body, updated) in writes {
            let dir = self.person_dir(&person)?.join("documents");
            fs::create_dir_all(&dir)?;
            let tmp = dir.join(format!("{id}.part"));
            fs::write(&tmp, &body)?;
            fs::File::open(&tmp)?.sync_all()?;
            fs::rename(&tmp, dir.join(&filename))?;
            let tx = self.db.transaction()?;
            tx.execute("INSERT INTO person_documents(id,person_id,title,filename,updated_at) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET title=excluded.title,updated_at=excluded.updated_at",params![id,person,title,filename,updated])?;
            tx.execute("DELETE FROM document_writes WHERE document_id=?1", [id])?;
            tx.commit()?;
        }
        Ok(())
    }
    pub fn delete_document(&mut self, person: &str, document: &str) -> Result<PersonSpace> {
        self.person(person, true)?;
        self.recover_document_writes()?;
        let doc = self
            .person_space(person)?
            .documents
            .into_iter()
            .find(|d| d.id == document)
            .context("文章不存在")?;
        trash::delete(
            self.person_dir(person)?
                .join("documents")
                .join(doc.filename),
        )?;
        self.db
            .execute("DELETE FROM person_documents WHERE id=?1", [document])?;
        self.person_space(person)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn photo(root: &Path) -> PathBuf {
        let p = root.join("scan.png");
        image::RgbImage::new(40, 60).save(&p).unwrap();
        p
    }
    fn create(s: &mut Store, name: &str, description: &str, duplicate: bool) -> Person {
        s.save_person(PersonDraft {
            name: name.into(),
            description: description.into(),
            allow_duplicate: duplicate,
            ..Default::default()
        })
        .unwrap()
    }
    #[test]
    fn duplicate_names_require_explicit_creation_and_distinct_description() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Store::open(tmp.path().join("lib")).unwrap();
        let a = create(&mut s, "小明", "", false);
        for (desc, allow) in [("", true), ("团A", false)] {
            assert!(s
                .save_person(PersonDraft {
                    name: "小明".into(),
                    description: desc.into(),
                    allow_duplicate: allow,
                    ..Default::default()
                })
                .is_err());
        }
        let b = create(&mut s, "小明", "团A", true);
        assert_ne!(a.id, b.id);
        assert!(s
            .save_person(PersonDraft {
                name: "小明".into(),
                description: "团A".into(),
                allow_duplicate: true,
                ..Default::default()
            })
            .is_err());
        let image = photo(tmp.path());
        s.import_file(&image, &Default::default()).unwrap();
        let c = s.list().unwrap().chekis.remove(0);
        let mut metadata = c.metadata;
        metadata.people_ids = Some(vec![a.id.clone(), b.id.clone()]);
        metadata.date = "2026-08-27".into();
        s.update(&c.id, metadata).unwrap();
        let c = s.list().unwrap().chekis.remove(0);
        assert_eq!(c.metadata.people.len(), 2);
        assert_eq!(c.metadata.people_ids.unwrap(), vec![a.id, b.id]);
    }
    #[test]
    fn rename_keeps_identity_aliases_original_path_and_deleted_person_can_restore() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Store::open(tmp.path().join("lib")).unwrap();
        let p = create(&mut s, "旧名", "", false);
        let image = photo(tmp.path());
        s.import_file(&image, &Default::default()).unwrap();
        let c = s.list().unwrap().chekis.remove(0);
        let mut m = c.metadata;
        m.people_ids = Some(vec![p.id.clone()]);
        m.date = "2026-08-27".into();
        s.update(&c.id, m).unwrap();
        let path = s.list().unwrap().chekis[0].assets[0].original_path.clone();
        let renamed = s
            .save_person(PersonDraft {
                id: Some(p.id.clone()),
                name: "新名".into(),
                ..Default::default()
            })
            .unwrap();
        assert!(renamed.aliases.contains(&"旧名".into()));
        assert_eq!(s.list().unwrap().chekis[0].assets[0].original_path, path);
        assert_eq!(s.list().unwrap().chekis[0].metadata.people, vec!["新名"]);
        s.person_trash(&p.id, false).unwrap();
        assert_eq!(
            s.list().unwrap().chekis[0].metadata.people_ids,
            Some(vec![p.id.clone()])
        );
        s.person_trash(&p.id, true).unwrap();
        assert!(s.person(&p.id, true).is_ok());
        let mut m = s.list().unwrap().chekis[0].metadata.clone();
        m.people_ids = Some(vec![]);
        s.update(&c.id, m).unwrap();
        assert_eq!(s.people().unwrap().len(), 1);
    }
    #[test]
    fn originals_and_markdown_survive_reopen_and_reject_external_edit_conflicts() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("lib");
        let mut s = Store::open(root.clone()).unwrap();
        let p = create(&mut s, "人物", "", false);
        let file = tmp.path().join("recording.bin");
        fs::write(&file, [0, 1, 255, 3]).unwrap();
        s.import_person_file(&p.id, &file).unwrap();
        let f = s.person_space(&p.id).unwrap().files.remove(0);
        assert_eq!(fs::read(&f.src).unwrap(), fs::read(&file).unwrap());
        let body = format!(
            "# 文章\n\n[附件](attachment:{})\n\n[收藏](cheki:test-id)",
            f.id
        );
        s.save_document(DocumentDraft {
            id: None,
            person_id: p.id.clone(),
            title: "一次演出".into(),
            body: body.clone(),
            revision: None,
        })
        .unwrap();
        let doc = s.person_space(&p.id).unwrap().documents.remove(0);
        drop(s);
        let mut s = Store::open(root).unwrap();
        assert_eq!(s.read_document(&p.id, &doc.id).unwrap().body, body);
        fs::write(
            s.person_dir(&p.id)
                .unwrap()
                .join("documents")
                .join(&doc.filename),
            "外部编辑",
        )
        .unwrap();
        assert!(s
            .save_document(DocumentDraft {
                id: Some(doc.id.clone()),
                person_id: p.id.clone(),
                title: doc.title,
                body: "不应该覆盖".into(),
                revision: Some(body)
            })
            .is_err());
        assert_eq!(s.read_document(&p.id, &doc.id).unwrap().body, "外部编辑");
    }
    #[test]
    fn attachment_rename_preserves_identity_bytes_and_references_and_rejects_collision() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("lib");
        let mut s = Store::open(root.clone()).unwrap();
        let p = create(&mut s, "人物", "", false);
        let source = tmp.path().join("audio.bin");
        fs::write(&source, [1, 2, 3, 255]).unwrap();
        s.import_person_file(&p.id, &source).unwrap();
        let f = s.person_space(&p.id).unwrap().files.remove(0);
        let body = format!("[录音](attachment:{})", f.id);
        s.save_document(DocumentDraft {
            id: None,
            person_id: p.id.clone(),
            title: "引用".into(),
            body: body.clone(),
            revision: None,
        })
        .unwrap();
        assert!(s.rename_person_file(&p.id, &f.id, "../bad.bin").is_err());
        assert!(s.rename_person_file(&p.id, &f.id, "audio.mp4").is_err());
        let dest = Path::new(&f.src).with_file_name("existing.bin");
        fs::write(&dest, [42]).unwrap();
        assert!(s.rename_person_file(&p.id, &f.id, "existing.bin").is_err());
        assert_eq!(fs::read(dest).unwrap(), vec![42]);
        let next = s
            .rename_person_file(&p.id, &f.id, "演出录音.bin")
            .unwrap()
            .files
            .remove(0);
        assert_eq!(next.id, f.id);
        assert_eq!(next.original_filename, "audio.bin");
        assert!(!Path::new(&f.src).exists());
        assert_eq!(fs::read(&next.src).unwrap(), vec![1, 2, 3, 255]);
        drop(s);
        let s = Store::open(root).unwrap();
        let space = s.person_space(&p.id).unwrap();
        assert_eq!(space.files[0].filename, "演出录音.bin");
        assert_eq!(
            s.read_document(&p.id, &space.documents[0].id).unwrap().body,
            body
        );
        assert!(source.is_file());
    }
    #[test]
    fn document_journal_recovers_after_interrupted_write() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("lib");
        let mut s = Store::open(root.clone()).unwrap();
        let p = create(&mut s, "人物", "", false);
        s.db.execute("INSERT INTO document_writes VALUES('doc',?1,'恢复标题','recover.md','恢复内容','2026-09-11')",[&p.id]).unwrap();
        drop(s);
        let s = Store::open(root).unwrap();
        assert_eq!(s.read_document(&p.id, "doc").unwrap().body, "恢复内容");
        assert_eq!(s.person_space(&p.id).unwrap().documents.len(), 1);
    }
    #[test]
    fn purge_person_trashes_files_but_keeps_chekis_and_their_originals() {
        if let Some(root) = std::env::var_os("CHEKI_PERSON_TRASH_TEST") {
            let root = PathBuf::from(root);
            let mut s = Store::open(root.join("lib")).unwrap();
            let p = create(&mut s, "人物", "", false);
            let image = photo(&root);
            s.import_file(&image, &Default::default()).unwrap();
            let c = s.list().unwrap().chekis.remove(0);
            let mut m = c.metadata;
            m.people_ids = Some(vec![p.id.clone()]);
            s.update(&c.id, m).unwrap();
            s.import_person_file(&p.id, &image).unwrap();
            s.save_document(DocumentDraft {
                id: None,
                person_id: p.id.clone(),
                title: "文章".into(),
                body: "正文".into(),
                revision: None,
            })
            .unwrap();
            assert!(s.person_purge(&p.id).is_err());
            s.person_trash(&p.id, false).unwrap();
            s.person_purge(&p.id).unwrap();
            let c = s.list().unwrap().chekis.remove(0);
            assert!(c.metadata.people.is_empty());
            assert!(Path::new(&c.assets[0].original_path).is_file());
            assert!(Path::new(&c.assets[0].src).is_file());
            assert!(s.people().unwrap().is_empty());
            assert_eq!(fs::read_dir(root.join("Trash/files")).unwrap().count(), 1);
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let r=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","storage::people::tests::purge_person_trashes_files_but_keeps_chekis_and_their_originals","--nocapture"]).env("XDG_DATA_HOME",tmp.path()).env("CHEKI_PERSON_TRASH_TEST",tmp.path()).output().unwrap();
        assert!(
            r.status.success(),
            "{} {}",
            String::from_utf8_lossy(&r.stdout),
            String::from_utf8_lossy(&r.stderr)
        );
    }
}
