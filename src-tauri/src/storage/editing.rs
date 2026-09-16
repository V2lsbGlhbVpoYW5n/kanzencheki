use super::geometry::{apply_crop, rotate_crop, validate_crop};
use super::*;
use image::imageops::FilterType;
use std::{io::Cursor, process::Command};

#[derive(Serialize, Deserialize)]
struct Preparation {
    source: PathBuf,
    staged: PathBuf,
    base: String,
    display: String,
}
#[derive(Serialize, Deserialize)]
struct Rotation {
    asset: String,
    source: PathBuf,
    staged: PathBuf,
    backup: PathBuf,
    base: String,
    display: String,
    old_caches: Vec<String>,
    crop: Option<Crop>,
    width: u32,
    height: u32,
    size: u64,
    fingerprint: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageView {
    pub path: String,
    pub original: bool,
    pub temporary: bool,
}
impl Store {
    fn editing_asset(&self, asset: &str) -> Result<Asset> {
        self.list()?
            .chekis
            .into_iter()
            .flat_map(|c| c.assets)
            .find(|a| a.id == asset)
            .context("影像不存在")
    }
    pub fn crop_preview(&self, asset: &str, crop: Crop) -> Result<Vec<u8>> {
        validate_crop(&crop)?;
        let a = self.editing_asset(asset)?;
        if a.base_src.is_empty() {
            bail!("本机完整预览不可用");
        }
        let image = apply_crop(image::open(&a.base_src)?.thumbnail(1000, 1000), Some(&crop))?;
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Jpeg)?;
        Ok(bytes.into_inner())
    }
    pub fn image_view(&self, asset: &str) -> Result<ImageView> {
        let a = self.editing_asset(asset)?;
        let source = a
            .renditions
            .iter()
            .find(|r| r.role == "original" && r.available);
        if let Some(source) = source {
            let path = self.path_for(source)?;
            if ["image/jpeg", "image/png", "image/webp"].contains(&source.mime_type.as_str()) {
                return Ok(ImageView {
                    path: path.to_string_lossy().into(),
                    original: true,
                    temporary: false,
                });
            }
            // TIFF cannot be displayed directly by WebKit: decode at original pixel dimensions.
            let dest = self.root.join("views").join(format!("{}.jpg", id()));
            let result = Command::new("magick")
                .arg(&path)
                .args(["-auto-orient", "-quality", "95"])
                .arg(&dest)
                .output();
            if result.is_ok_and(|r| r.status.success()) && dest.is_file() {
                return Ok(ImageView {
                    path: dest.to_string_lossy().into(),
                    original: true,
                    temporary: true,
                });
            }
            let _ = fs::remove_file(dest);
        }
        let path = if !a.base_src.is_empty() {
            a.base_src
        } else {
            a.src
        };
        if !Path::new(&path).is_file() {
            bail!("原件与本机预览均不可用");
        }
        Ok(ImageView {
            path,
            original: false,
            temporary: false,
        })
    }
    pub fn release_view(&self, path: &str) -> Result<()> {
        let p = Path::new(path);
        if p.parent() != Some(self.root.join("views").as_path()) {
            bail!("无效临时查看文件");
        }
        if p.is_file() {
            fs::remove_file(p)?;
        }
        Ok(())
    }
    pub fn rotate(&mut self, asset: &str) -> Result<Library> {
        self.recover_rotations()?;
        let journal = self.root.join("rotations").join(format!("{asset}.json"));
        if journal.exists() {
            bail!("上次旋转尚未完成，请连接原件目录");
        }
        let a = self.editing_asset(asset)?;
        let source = a
            .renditions
            .iter()
            .find(|r| r.role == "original" && r.available)
            .context("原件离线，无法旋转；不会保存待处理任务")?;
        let source = self.path_for(source)?;
        let ext = source
            .extension()
            .and_then(|e| e.to_str())
            .context("原件扩展名无效")?;
        let token = id();
        let staged = source.with_file_name(format!(".cheki-{token}.{ext}"));
        let backup = source.with_file_name(format!(".cheki-{token}.backup"));
        let base = format!("previews/{asset}-base-{token}.jpg");
        let display = format!("previews/{asset}-display-{token}.jpg");
        let crop = a.crop.as_ref().map(rotate_crop);
        let preparing = journal.with_extension("prepare");
        let mut record = fs::File::create(&preparing)?;
        serde_json::to_writer(
            &mut record,
            &Preparation {
                source: source.clone(),
                staged: staged.clone(),
                base: base.clone(),
                display: display.clone(),
            },
        )?;
        record.sync_all()?;
        let prepare = (|| -> Result<Rotation> {
            let result = Command::new("magick")
                .arg(&source)
                .args([
                    "-auto-orient",
                    "-rotate",
                    "90",
                    "+repage",
                    "-orient",
                    "TopLeft",
                ])
                .arg(&staged)
                .output()
                .context("旋转需要 ImageMagick")?;
            if !result.status.success() {
                bail!("旋转失败：{}", String::from_utf8_lossy(&result.stderr));
            }
            let (width, height) = image::image_dimensions(&staged)?;
            let size = fs::metadata(&staged)?.len();
            fs::File::open(&staged)?.sync_all()?;
            make_preview(&staged, &self.root.join(&base))?;
            let image = apply_crop(image::open(self.root.join(&base))?, crop.as_ref())?;
            image.to_rgb8().save(self.root.join(&display))?;
            for p in [&base, &display] {
                fs::File::open(self.root.join(p))?.sync_all()?;
            }
            let small = image.resize_exact(9, 8, FilterType::Triangle).to_luma8();
            let mut hash = 0u64;
            for y in 0..8 {
                for x in 0..8 {
                    hash = (hash << 1)
                        | u64::from(small.get_pixel(x, y)[0] > small.get_pixel(x + 1, y)[0]);
                }
            }
            Ok(Rotation {
                asset: asset.into(),
                source: source.clone(),
                staged: staged.clone(),
                backup,
                base: base.clone(),
                display: display.clone(),
                old_caches: a
                    .renditions
                    .iter()
                    .filter(|r| r.role != "original")
                    .map(|r| r.relative_path.clone())
                    .collect(),
                crop,
                width,
                height,
                size,
                fingerprint: format!("{hash:016x}"),
            })
        })();
        let prepared = match prepare {
            Ok(p) => p,
            Err(e) => {
                for p in [staged, self.root.join(base), self.root.join(display)] {
                    let _ = fs::remove_file(p);
                }
                let _ = fs::remove_file(&preparing);
                return Err(e);
            }
        };
        let temp = journal.with_extension("part");
        let mut file = fs::File::create(&temp)?;
        serde_json::to_writer(&mut file, &prepared)?;
        file.sync_all()?;
        fs::rename(temp, &journal)?;
        fs::remove_file(preparing)?;
        self.finish_rotation(&prepared)?;
        fs::remove_file(journal)?;
        self.list()
    }
    fn finish_rotation(&mut self, r: &Rotation) -> Result<()> {
        if r.staged.is_file() {
            if !r.backup.exists() {
                fs::rename(&r.source, &r.backup)?;
            }
            fs::rename(&r.staged, &r.source)?;
        }
        if !r.source.is_file() {
            bail!("旋转原件暂不可用");
        }
        let tx = self.db.transaction()?;
        tx.execute("UPDATE assets SET width=?2,height=?3,crop_json=?4,fingerprint=?5,preview_error=NULL WHERE id=?1",params![r.asset,r.width,r.height,r.crop.as_ref().map(serde_json::to_string).transpose()?,r.fingerprint])?;
        tx.execute("UPDATE renditions SET width=?2,height=?3,byte_size=?4 WHERE asset_id=?1 AND role='original'",params![r.asset,r.width,r.height,r.size as i64])?;
        for (role, path) in [("base", &r.base), ("display", &r.display)] {
            let p = self.root.join(path);
            let (w, h) = image::image_dimensions(&p)?;
            tx.execute("INSERT INTO renditions(id,asset_id,role,relative_path,mime_type,byte_size,width,height) VALUES(?1,?2,?3,?4,'image/jpeg',?5,?6,?7) ON CONFLICT(asset_id,role) DO UPDATE SET relative_path=excluded.relative_path,byte_size=excluded.byte_size,width=excluded.width,height=excluded.height",params![id(),r.asset,role,path,fs::metadata(p)?.len() as i64,w,h])?;
        }
        tx.commit()?;
        if r.backup.is_file() {
            fs::remove_file(&r.backup)?;
        }
        for old in &r.old_caches {
            if old.starts_with("previews/") && old != &r.base && old != &r.display {
                let _ = fs::remove_file(self.root.join(old));
            }
        }
        Ok(())
    }
    pub fn recover_rotations(&mut self) -> Result<()> {
        for entry in fs::read_dir(self.root.join("rotations"))? {
            let path = entry?.path();
            if !path.extension().is_some_and(|e| e == "prepare") {
                continue;
            }
            if !path.with_extension("json").exists() {
                let p: Preparation = serde_json::from_reader(fs::File::open(&path)?)?;
                if !p.source.exists() && !p.staged.exists() && !p.source.starts_with(&self.root) {
                    continue;
                }
                for file in [p.staged, self.root.join(p.base), self.root.join(p.display)] {
                    if file.is_file() {
                        fs::remove_file(file)?;
                    }
                }
            }
            fs::remove_file(path)?;
        }
        for entry in fs::read_dir(self.root.join("rotations"))? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "json") {
                let record: Rotation = serde_json::from_reader(fs::File::open(&path)?)?;
                // A disconnected disk must not prevent browsing the library. This is recovery of an
                // already-started filesystem transaction, never a queued offline edit.
                if !record.source.exists() && !record.staged.exists() {
                    continue;
                }
                self.finish_rotation(&record)?;
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_preparation_cleans_only_new_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("lib");
        let store = Store::open(root.clone()).unwrap();
        let source = root.join("original.png");
        fs::write(&source, b"original bytes").unwrap();
        let staged = source.with_file_name(".cheki-interrupted.png");
        let base = "previews/interrupted-base.jpg".to_string();
        let display = "previews/interrupted-display.jpg".to_string();
        for file in [&staged, &root.join(&base), &root.join(&display)] {
            fs::write(file, b"partial").unwrap();
        }
        let manifest = root.join("rotations/interrupted.prepare");
        serde_json::to_writer(
            fs::File::create(&manifest).unwrap(),
            &Preparation {
                source: source.clone(),
                staged: staged.clone(),
                base: base.clone(),
                display: display.clone(),
            },
        )
        .unwrap();
        drop(store);
        let _store = Store::open(root.clone()).unwrap();
        assert_eq!(fs::read(source).unwrap(), b"original bytes");
        assert!(!staged.exists());
        assert!(!root.join(base).exists());
        assert!(!root.join(display).exists());
        assert!(!manifest.exists());
    }
    #[test]
    fn rotation_updates_original_cache_crop_and_survives_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("lib");
        let source = temp.path().join("asymmetric.png");
        image::RgbImage::from_fn(40, 60, |x, y| image::Rgb([x as u8, y as u8, 0]))
            .save(&source)
            .unwrap();
        let mut store = Store::open(root.clone()).unwrap();
        store.import_file(&source, &Default::default()).unwrap();
        let a = store.list().unwrap().chekis[0].assets[0].clone();
        store
            .crop(
                &a.id,
                Some(Crop {
                    x: 0.1,
                    y: 0.2,
                    w: 0.4,
                    h: 0.5,
                    ..Default::default()
                }),
            )
            .unwrap();
        store.rotate(&a.id).unwrap();
        let b = store.list().unwrap().chekis[0].assets[0].clone();
        assert_eq!((b.width, b.height), (Some(60), Some(40)));
        let pixels = image::open(&b.original_path).unwrap().to_rgb8();
        assert_eq!(pixels.get_pixel(59, 0).0, [0, 0, 0]);
        assert!((b.crop.unwrap().x - 0.3).abs() < 1e-8);
        assert_ne!(a.src, b.src);
        assert!(!Path::new(&a.src).exists());
        drop(store);
        let store = Store::open(root).unwrap();
        assert_eq!(store.list().unwrap().chekis[0].assets[0].width, Some(60));
        assert_eq!(image::image_dimensions(source).unwrap(), (40, 60));
    }
    #[test]
    fn rotation_journal_recovers_at_every_commit_boundary() {
        for phase in 0..4 {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("lib");
            let input = temp.path().join("input.png");
            image::RgbImage::from_fn(40, 60, |x, y| image::Rgb([x as u8, y as u8, 0]))
                .save(&input)
                .unwrap();
            let mut store = Store::open(root.clone()).unwrap();
            store.import_file(&input, &Default::default()).unwrap();
            let a = store.list().unwrap().chekis[0].assets[0].clone();
            let source = PathBuf::from(&a.original_path);
            let staged = source.with_file_name("staged.png");
            let backup = source.with_file_name("backup.png");
            let rotated = image::open(&source).unwrap().rotate90();
            rotated.save(&staged).unwrap();
            let base = "previews/recovery-base.jpg".to_string();
            let display = "previews/recovery-display.jpg".to_string();
            rotated.to_rgb8().save(root.join(&base)).unwrap();
            rotated.to_rgb8().save(root.join(&display)).unwrap();
            let record = Rotation {
                asset: a.id.clone(),
                source: source.clone(),
                staged: staged.clone(),
                backup: backup.clone(),
                base,
                display,
                old_caches: a
                    .renditions
                    .iter()
                    .filter(|r| r.role != "original")
                    .map(|r| r.relative_path.clone())
                    .collect(),
                crop: None,
                width: 60,
                height: 40,
                size: fs::metadata(&staged).unwrap().len(),
                fingerprint: "recovery".into(),
            };
            let journal = root.join("rotations").join(format!("{}.json", a.id));
            serde_json::to_writer(fs::File::create(&journal).unwrap(), &record).unwrap();
            if phase >= 1 {
                fs::rename(&source, &backup).unwrap();
            }
            if phase >= 2 {
                fs::rename(&staged, &source).unwrap();
            }
            if phase == 3 {
                store.finish_rotation(&record).unwrap();
            }
            drop(store);
            let store = Store::open(root).unwrap();
            let next = store.list().unwrap().chekis[0].assets[0].clone();
            assert_eq!((next.width, next.height), (Some(60), Some(40)));
            assert_eq!(
                image::open(&source).unwrap().to_rgb8().get_pixel(59, 39).0,
                [39, 0, 0]
            );
            assert!(!journal.exists());
            assert!(!backup.exists());
            assert!(!staged.exists());
            assert!(Path::new(&next.src).is_file());
        }
    }
    #[test]
    fn tiff_rotation_preserves_bit_depth_and_full_view_is_transient() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("lib");
        let input = temp.path().join("scan.tiff");
        let image = image::ImageBuffer::<image::Rgb<u16>, Vec<u16>>::from_fn(20, 30, |x, y| {
            image::Rgb([(x * 1000) as u16, (y * 1000) as u16, 321])
        });
        image.save(&input).unwrap();
        let mut store = Store::open(root).unwrap();
        store.import_file(&input, &Default::default()).unwrap();
        let a = store.list().unwrap().chekis[0].assets[0].clone();
        let view = store.image_view(&a.id).unwrap();
        assert!(view.original && view.temporary);
        assert_eq!(image::image_dimensions(&view.path).unwrap(), (20, 30));
        store.release_view(&view.path).unwrap();
        assert!(!Path::new(&view.path).exists());
        assert!(store.release_view(&a.original_path).is_err());
        store.rotate(&a.id).unwrap();
        let output = image::open(&a.original_path).unwrap();
        assert_eq!(output.color(), image::ColorType::Rgb16);
        assert_eq!(output.to_rgb16().get_pixel(29, 19).0, [19000, 0, 321]);
    }
    #[test]
    fn perspective_edit_is_offline_persistent_and_leaves_original_bytes_unchanged() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("lib");
        let input = temp.path().join("scan.png");
        image::RgbImage::new(100, 150).save(&input).unwrap();
        let mut store = Store::open(root.clone()).unwrap();
        store.import_file(&input, &Default::default()).unwrap();
        let a = store.list().unwrap().chekis[0].assets[0].clone();
        let original = fs::read(&a.original_path).unwrap();
        let c = Crop {
            x: 0.,
            y: 0.,
            w: 1.,
            h: 1.,
            quad: Some([
                Point { x: 0.1, y: 0.1 },
                Point { x: 0.9, y: 0.2 },
                Point { x: 0.8, y: 0.9 },
                Point { x: 0.2, y: 0.8 },
            ]),
            ratio: Some(1.),
        };
        let preview = store.crop_preview(&a.id, c.clone()).unwrap();
        assert!(!preview.is_empty());
        store.crop(&a.id, Some(c)).unwrap();
        assert_eq!(fs::read(&a.original_path).unwrap(), original);
        fs::rename(&a.original_path, temp.path().join("offline.png")).unwrap();
        drop(store);
        let mut store = Store::open(root).unwrap();
        let a = store.list().unwrap().chekis[0].assets[0].clone();
        assert!(a.crop.as_ref().unwrap().quad.is_some());
        let d = image::image_dimensions(&a.src).unwrap();
        assert_eq!(d.0, d.1);
        store.crop(&a.id, None).unwrap();
        let reset = store.list().unwrap().chekis[0].assets[0].clone();
        assert_eq!(image::image_dimensions(reset.src).unwrap(), (100, 150));
    }
    #[test]
    fn offline_rotation_does_not_queue_and_view_uses_base() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("lib");
        let disk = temp.path().join("disk");
        fs::create_dir(&disk).unwrap();
        let path = disk.join("scan.png");
        image::RgbImage::new(40, 60).save(&path).unwrap();
        let mut store = Store::open(root.clone()).unwrap();
        store.add_location(&disk, None).unwrap();
        store
            .import_file(
                &path,
                &ImportOptions {
                    reference: true,
                    ..Default::default()
                },
            )
            .unwrap();
        let a = store.list().unwrap().chekis[0].assets[0].clone();
        fs::rename(&disk, temp.path().join("offline")).unwrap();
        assert!(store.rotate(&a.id).is_err());
        assert_eq!(fs::read_dir(root.join("rotations")).unwrap().count(), 0);
        assert!(!store.image_view(&a.id).unwrap().original);
    }
}
