use super::*;

#[derive(Clone)]
pub struct DetectionJob {
    pub id: String,
    revision: i64,
    asset: String,
    pub path: String,
}
#[derive(Clone)]
pub struct DetectionReference {
    pub person: String,
    pub path: String,
}
impl Store {
    pub(super) fn migrate_detection(&mut self) -> Result<()> {
        // Inspect columns as well as version so old-schema migration fixtures remain valid.
        let present: bool = self.db.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('chekis') WHERE name='detection_revision')", [], |r| r.get(0))?;
        if !present {
            self.db.execute_batch(
                "BEGIN;
                ALTER TABLE chekis ADD COLUMN detection_revision INTEGER NOT NULL DEFAULT 0;
                ALTER TABLE chekis ADD COLUMN review_faces INTEGER;
                COMMIT;",
            )?;
        }
        self.db.execute_batch("BEGIN;
            CREATE TRIGGER IF NOT EXISTS detection_manual_edit AFTER UPDATE OF date,event,shot_type,notes,favorite,group_name,cover_asset_id,deleted_at ON chekis BEGIN
                UPDATE chekis SET detection_revision=detection_revision+1 WHERE id=NEW.id;
            END;
            CREATE TRIGGER IF NOT EXISTS detection_people_add AFTER INSERT ON cheki_people BEGIN
                UPDATE chekis SET detection_revision=detection_revision+1 WHERE id=NEW.cheki_id;
            END;
            CREATE TRIGGER IF NOT EXISTS detection_people_remove AFTER DELETE ON cheki_people BEGIN
                UPDATE chekis SET detection_revision=detection_revision+1 WHERE id=OLD.cheki_id;
            END;
            CREATE TRIGGER IF NOT EXISTS detection_asset_add AFTER INSERT ON cheki_assets BEGIN
                UPDATE chekis SET detection_revision=detection_revision+1 WHERE id=NEW.cheki_id;
            END;
            CREATE TRIGGER IF NOT EXISTS detection_asset_remove AFTER DELETE ON cheki_assets BEGIN
                UPDATE chekis SET detection_revision=detection_revision+1 WHERE id=OLD.cheki_id;
            END;
            PRAGMA user_version=5; COMMIT;")?;
        Ok(())
    }
    pub fn detection_jobs(&self, ids: &[String]) -> Result<Vec<DetectionJob>> {
        let mut jobs = vec![];
        for c in self.list()?.chekis {
            if !ids.contains(&c.id)
                || c.deleted_at.is_some()
                || c.metadata.shot_type != "其他"
                || !c.metadata.people.is_empty()
                || !c.metadata.group.is_empty()
                || c.review_faces.is_some()
            {
                continue;
            }
            let a = c
                .assets
                .iter()
                .find(|a| Some(&a.id) == c.cover_asset_id.as_ref())
                .or(c.assets.first());
            if let Some(a) = a.filter(|a| !a.src.is_empty()) {
                jobs.push(DetectionJob {
                    revision: self.db.query_row(
                        "SELECT detection_revision FROM chekis WHERE id=?1",
                        [&c.id],
                        |r| r.get(0),
                    )?,
                    id: c.id,
                    asset: a.id.clone(),
                    path: a.src.clone(),
                });
            }
        }
        Ok(jobs)
    }
    pub fn detection_references(&self) -> Result<Vec<DetectionReference>> {
        let mut references = vec![];
        for c in self.list()?.chekis {
            if c.deleted_at.is_some()
                || c.review_faces.is_some()
                || c.metadata.shot_type != "solo"
                || c.metadata.people_ids.as_ref().map(Vec::len) != Some(1)
            {
                continue;
            }
            let asset = c
                .assets
                .iter()
                .find(|a| Some(&a.id) == c.cover_asset_id.as_ref())
                .or(c.assets.first());
            if let Some(asset) = asset.filter(|a| !a.src.is_empty()) {
                references.push(DetectionReference {
                    person: c.metadata.people_ids.as_ref().unwrap()[0].clone(),
                    path: asset.src.clone(),
                });
            }
        }
        Ok(references)
    }
    pub fn apply_detection(
        &mut self,
        job: &DetectionJob,
        count: usize,
        people: &[String],
    ) -> Result<bool> {
        let kind = match count {
            0 => return Ok(false),
            1 => "solo",
            2 => "2 shot",
            3..=4 => "多人切",
            _ => "团切",
        };
        // All checks and the conditional update run under the Store mutex. A replaced crop,
        // deleted/merged asset, or manual edit invalidates an in-flight result.
        let current = self.detection_jobs(&[job.id.clone()])?;
        if !current
            .iter()
            .any(|j| j.revision == job.revision && j.asset == job.asset && j.path == job.path)
        {
            return Ok(false);
        }
        let tx = self.db.transaction()?;
        let changed = tx.execute("UPDATE chekis SET shot_type=?2,review_faces=?3 WHERE id=?1 AND detection_revision=?4 AND shot_type='其他' AND deleted_at IS NULL AND NOT EXISTS(SELECT 1 FROM cheki_people WHERE cheki_id=?1)", params![job.id,kind,count as i64,job.revision])? == 1;
        if changed && kind != "团切" {
            for (position, person) in people.iter().enumerate() {
                let active: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM people WHERE id=?1 AND deleted_at IS NULL)",
                    [person],
                    |r| r.get(0),
                )?;
                if active {
                    tx.execute(
                        "INSERT OR IGNORE INTO cheki_people(cheki_id,person_id,position) VALUES(?1,?2,?3)",
                        params![job.id, person, position as i64],
                    )?;
                }
            }
        }
        tx.commit()?;
        Ok(changed)
    }
    pub fn confirm_detection(&mut self, id: &str) -> Result<Library> {
        self.exists(id)?;
        self.db.execute("UPDATE chekis SET review_faces=NULL,detection_revision=detection_revision+1 WHERE id=?1", [id])?;
        self.list()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Store, String) {
        let temp = tempfile::tempdir().unwrap();
        let image = temp.path().join("input.png");
        image::RgbImage::new(80, 100).save(&image).unwrap();
        let mut store = Store::open(temp.path().join("lib")).unwrap();
        store.import_file(&image, &Default::default()).unwrap();
        let id = store.list().unwrap().chekis[0].id.clone();
        (temp, store, id)
    }
    #[test]
    fn counts_are_reviewable_persistent_and_confirmation_does_not_complete_inbox() {
        for (count, kind) in [
            (0, "其他"),
            (1, "solo"),
            (2, "2 shot"),
            (4, "多人切"),
            (5, "团切"),
        ] {
            let (_t, mut s, id) = fixture();
            let job = s.detection_jobs(&[id.clone()]).unwrap().remove(0);
            assert_eq!(s.apply_detection(&job, count, &[]).unwrap(), count > 0);
            let root = s.root.clone();
            drop(s);
            let mut s = Store::open(root).unwrap();
            let c = s.list().unwrap().chekis.remove(0);
            assert_eq!(c.metadata.shot_type, kind);
            assert_eq!(c.review_faces, count.checked_sub(1).map(|_| count as u32));
            let lib = if count == 2 {
                let mut metadata = c.metadata;
                metadata.shot_type = "其他".into();
                s.update(&id, metadata).unwrap()
            } else {
                s.confirm_detection(&id).unwrap()
            };
            assert!(lib.chekis[0].review_faces.is_none());
            assert!(lib.chekis[0].metadata.people.is_empty());
        }
    }
    #[test]
    fn assigned_people_crops_and_asset_removal_invalidate_results() {
        let (_t, mut s, id) = fixture();
        let job = s.detection_jobs(&[id.clone()]).unwrap().remove(0);
        let mut m = s.list().unwrap().chekis[0].metadata.clone();
        m.people_ids = None;
        m.people = vec!["手动人物".into()];
        s.update(&id, m).unwrap();
        assert!(!s.apply_detection(&job, 2, &[]).unwrap());
        assert_eq!(
            s.list().unwrap().chekis[0].metadata.people,
            vec!["手动人物"]
        );

        let (_t, mut s, id) = fixture();
        let job = s.detection_jobs(&[id.clone()]).unwrap().remove(0);
        s.crop(
            &job.asset,
            Some(Crop {
                x: 0.1,
                y: 0.1,
                w: 0.8,
                h: 0.8,
                ..Default::default()
            }),
        )
        .unwrap();
        assert!(!s.apply_detection(&job, 2, &[]).unwrap());
        let job = s.detection_jobs(&[id.clone()]).unwrap().remove(0);
        s.db.execute("DELETE FROM cheki_assets WHERE cheki_id=?1", [&id])
            .unwrap();
        assert!(!s.apply_detection(&job, 5, &[]).unwrap());
    }
    #[test]
    fn stale_jobs_cannot_override_manual_edits_or_removed_assets() {
        let (_t, mut s, id) = fixture();
        let job = s.detection_jobs(&[id.clone()]).unwrap().remove(0);
        let mut m = s.list().unwrap().chekis[0].metadata.clone();
        m.shot_type = "solo".into();
        s.update(&id, m.clone()).unwrap();
        m.shot_type = "其他".into();
        s.update(&id, m).unwrap();
        assert!(!s.apply_detection(&job, 5, &[]).unwrap());
        let job = s.detection_jobs(&[id.clone()]).unwrap().remove(0);
        s.db.execute("UPDATE chekis SET deleted_at='deleted' WHERE id=?1", [id])
            .unwrap();
        assert!(!s.apply_detection(&job, 2, &[]).unwrap());
    }
    #[test]
    fn high_confidence_people_are_reviewable_and_groups_never_link_people() {
        let (_t, mut s, id) = fixture();
        let person = s
            .save_person(PersonDraft {
                name: "测试人物".into(),
                ..Default::default()
            })
            .unwrap();
        let job = s.detection_jobs(&[id.clone()]).unwrap().remove(0);
        assert!(s
            .apply_detection(&job, 2, std::slice::from_ref(&person.id))
            .unwrap());
        let c = s.list().unwrap().chekis.remove(0);
        assert_eq!(c.metadata.people_ids, Some(vec![person.id.clone()]));
        assert_eq!(c.metadata.people, vec!["测试人物"]);
        assert_eq!(c.review_faces, Some(2));

        let (_t, mut s, id) = fixture();
        let person = s
            .save_person(PersonDraft {
                name: "团切不应关联".into(),
                ..Default::default()
            })
            .unwrap();
        let job = s.detection_jobs(&[id]).unwrap().remove(0);
        assert!(s.apply_detection(&job, 5, &[person.id]).unwrap());
        assert!(s.list().unwrap().chekis[0].metadata.people.is_empty());
    }
}
