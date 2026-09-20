//! Offline CPU face counting and conservative identity suggestions.
use crate::storage::{DetectionJob, DetectionReference};
use crate::{progress, Backend};
use anyhow::{Context, Result};
use image::{imageops::FilterType, RgbImage};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tract_onnx::prelude::*;

static WORKER: Mutex<()> = Mutex::new(());
const FACE_SIZE: u32 = 112;
// Calibrated conservatively on unaligned face crops: same-person public samples
// score about 0.31 while a different person scores about 0.13.
const MATCH_THRESHOLD: f32 = 0.30;
const MATCH_MARGIN: f32 = 0.10;
const REFERENCES_PER_PERSON: usize = 5;

type FaceModel = TypedRunnableModel<TypedModel>;

fn detector() -> Result<Box<dyn rustface::Detector>> {
    let model = rustface::read_model(std::io::Cursor::new(
        include_bytes!("../models/seeta_fd_frontal_v1.0.bin").as_slice(),
    ))?;
    let mut detector = rustface::create_detector_with_model(model);
    detector.set_min_face_size(24);
    detector.set_score_thresh(4.5);
    detector.set_pyramid_scale_factor(0.8);
    detector.set_slide_window_step(4, 4);
    Ok(detector)
}

fn recognizer(path: &Path) -> Result<FaceModel> {
    Ok(tract_onnx::onnx()
        .model_for_path(path)
        .context("无法读取 MobileFaceNet 模型")?
        .into_optimized()
        .context("无法优化 MobileFaceNet 模型")?
        .into_runnable()
        .context("无法初始化 MobileFaceNet 模型")?)
}

fn model_path(app: &tauri::AppHandle) -> PathBuf {
    let bundled = app
        .path()
        .resource_dir()
        .ok()
        .map(|p| p.join("models/mobile_facenet/mobile_facenet.onnx"));
    bundled.filter(|p| p.is_file()).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models/mobile_facenet/mobile_facenet.onnx")
    })
}

fn face_crops(detector: &mut dyn rustface::Detector, path: &str) -> Result<Vec<RgbImage>> {
    let image = image::open(path)
        .context("无法读取本机浏览图")?
        .thumbnail(960, 960);
    let gray = image.to_luma8();
    let rgb = image.to_rgb8();
    let faces = detector.detect(&rustface::ImageData::new(
        gray.as_raw(),
        gray.width(),
        gray.height(),
    ));
    Ok(faces
        .iter()
        .filter_map(|face| {
            let b = face.bbox();
            let side = ((b.width().max(b.height()) as f32) * 1.35).round() as i32;
            if side < 16 {
                return None;
            }
            let cx = b.x() + b.width() as i32 / 2;
            let cy = b.y() + b.height() as i32 / 2;
            let x = (cx - side / 2).clamp(0, rgb.width().saturating_sub(1) as i32);
            let y = (cy - side / 2).clamp(0, rgb.height().saturating_sub(1) as i32);
            let width = (side as u32).min(rgb.width() - x as u32);
            let height = (side as u32).min(rgb.height() - y as u32);
            if width < 16 || height < 16 {
                return None;
            }
            Some(image::imageops::resize(
                &image::imageops::crop_imm(&rgb, x as u32, y as u32, width, height).to_image(),
                FACE_SIZE,
                FACE_SIZE,
                FilterType::Triangle,
            ))
        })
        .collect())
}

fn embedding(model: &FaceModel, face: &RgbImage) -> Result<Vec<f32>> {
    let mut values = vec![0.0f32; (3 * FACE_SIZE * FACE_SIZE) as usize];
    let plane = (FACE_SIZE * FACE_SIZE) as usize;
    for (x, y, pixel) in face.enumerate_pixels() {
        let index = (y * FACE_SIZE + x) as usize;
        for channel in 0..3 {
            values[channel * plane + index] = pixel[channel] as f32 / 255.0;
        }
    }
    let input = Tensor::from_shape(&[1, 3, FACE_SIZE as usize, FACE_SIZE as usize], &values)?;
    let output = model.run(tvec![input.clone().into(), input.into()])?;
    let view = output[0].to_array_view::<f32>()?;
    let mut result: Vec<f32> = view.iter().take(128).copied().collect();
    let length = result.iter().map(|v| v * v).sum::<f32>().sqrt();
    if result.len() != 128 || !length.is_finite() || length < 1e-6 {
        anyhow::bail!("人脸特征结果无效");
    }
    for value in &mut result {
        *value /= length;
    }
    Ok(result)
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

fn reference_embeddings(
    detector: &mut dyn rustface::Detector,
    model: &FaceModel,
    references: &[DetectionReference],
) -> HashMap<String, Vec<Vec<f32>>> {
    let mut result: HashMap<String, Vec<Vec<f32>>> = HashMap::new();
    for reference in references {
        if result.get(&reference.person).map(Vec::len).unwrap_or(0) >= REFERENCES_PER_PERSON {
            continue;
        }
        let Ok(faces) = face_crops(detector, &reference.path) else {
            continue;
        };
        // A confirmed solo with exactly one visible face is the least ambiguous reference.
        if faces.len() == 1 {
            if let Ok(vector) = embedding(model, &faces[0]) {
                result
                    .entry(reference.person.clone())
                    .or_default()
                    .push(vector);
            }
        }
    }
    result
}

fn match_people(
    model: &FaceModel,
    faces: &[RgbImage],
    references: &HashMap<String, Vec<Vec<f32>>>,
) -> Vec<String> {
    let mut matched: Vec<String> = vec![];
    for face in faces {
        let Ok(vector) = embedding(model, face) else {
            continue;
        };
        let mut scores: Vec<(&String, f32)> = references
            .iter()
            .map(|(person, samples)| {
                (
                    person,
                    samples
                        .iter()
                        .map(|sample| cosine(&vector, sample))
                        .fold(f32::NEG_INFINITY, f32::max),
                )
            })
            .collect();
        scores.sort_by(|a, b| b.1.total_cmp(&a.1));
        if let Some((person, best)) = scores.first() {
            let second = scores.get(1).map(|s| s.1).unwrap_or(f32::NEG_INFINITY);
            if *best >= MATCH_THRESHOLD
                && *best - second >= MATCH_MARGIN
                && !matched.contains(*person)
            {
                matched.push((*person).clone());
            }
        }
    }
    matched
}

pub fn start(
    app: tauri::AppHandle,
    backend: Backend,
    jobs: Vec<DetectionJob>,
    reference_jobs: Vec<DetectionReference>,
) {
    if jobs.is_empty() {
        return;
    }
    let task = uuid::Uuid::new_v4().to_string();
    let model_path = model_path(&app);
    progress(
        &app,
        &task,
        "本地人脸识别",
        0,
        jobs.len(),
        "running",
        "等待 CPU 后台识别",
    );
    tauri::async_runtime::spawn_blocking(move || {
        // One CPU worker across concurrent imports; never hold the library mutex during inference.
        let _worker = WORKER.lock().unwrap_or_else(|e| e.into_inner());
        let (mut detector, recognizer) = match (detector(), recognizer(&model_path)) {
            (Ok(d), Ok(r)) => (d, r),
            (Err(e), _) | (_, Err(e)) => {
                progress(
                    &app,
                    &task,
                    "本地人脸识别",
                    0,
                    jobs.len(),
                    "error",
                    &format!("模型加载失败：{e}"),
                );
                return;
            }
        };
        let references = reference_embeddings(detector.as_mut(), &recognizer, &reference_jobs);
        let reference_people = references.len();
        let mut changed = 0;
        let mut matched = 0;
        let mut errors = vec![];
        for (i, job) in jobs.iter().enumerate() {
            let result = face_crops(detector.as_mut(), &job.path).and_then(|faces| {
                let people = if faces.len() < 5 {
                    match_people(&recognizer, &faces, &references)
                } else {
                    vec![]
                };
                let count = faces.len();
                backend
                    .0
                    .lock()
                    .map_err(|_| anyhow::anyhow!("图库锁不可用"))?
                    .apply_detection(job, count, &people)
                    .map(|changed| (changed, people.len()))
            });
            match result {
                Ok((true, people)) => {
                    changed += 1;
                    matched += people;
                    let _ = app.emit("library-changed", ());
                }
                Ok((false, _)) => {}
                Err(e) => errors.push(format!("{}：{e}", job.id)),
            }
            progress(
                &app,
                &task,
                "本地人脸识别",
                i + 1,
                jobs.len(),
                "running",
                &format!("{changed} 张待核验 · 匹配 {matched} 个人物"),
            );
        }
        progress(
            &app,
            &task,
            "本地人脸识别",
            jobs.len(),
            jobs.len(),
            if errors.is_empty() { "done" } else { "error" },
            &if errors.is_empty() {
                format!(
                    "{changed} 张收藏待核验；从 {reference_people} 个人物的单人收藏中匹配到 {matched} 个建议"
                )
            } else {
                errors.join("；")
            },
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_models_load_and_blank_image_has_no_faces() {
        let mut detector = detector().unwrap();
        let pixels = vec![128; 100 * 100];
        assert!(detector
            .detect(&rustface::ImageData::new(&pixels, 100, 100))
            .is_empty());
        let model = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("models/mobile_facenet/mobile_facenet.onnx");
        let model = recognizer(&model).unwrap();
        assert_eq!(
            embedding(&model, &RgbImage::new(FACE_SIZE, FACE_SIZE))
                .unwrap()
                .len(),
            128
        );
    }
    #[test]
    fn matching_requires_absolute_score_and_margin() {
        let refs = [
            ("a".to_string(), vec![vec![1.0, 0.0]]),
            (
                "b".to_string(),
                vec![vec![std::f32::consts::FRAC_1_SQRT_2; 2]],
            ),
        ];
        let scores = |v: &[f32]| {
            let mut values: Vec<_> = refs.iter().map(|(id, r)| (id, cosine(v, &r[0]))).collect();
            values.sort_by(|a, b| b.1.total_cmp(&a.1));
            values
        };
        let strong = scores(&[1.0, 0.0]);
        assert!(strong[0].1 >= MATCH_THRESHOLD && strong[0].1 - strong[1].1 >= MATCH_MARGIN);
        let ambiguous = scores(&[0.923_879_5, 0.382_683_4]);
        assert!(ambiguous[0].1 - ambiguous[1].1 < MATCH_MARGIN);
    }
    #[test]
    #[ignore = "Set CHEKI_FACE_A/B/C to two same-person and one different-person photos"]
    fn reports_similarity_for_external_calibration_images() {
        let mut detector = detector().unwrap();
        let model = recognizer(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("models/mobile_facenet/mobile_facenet.onnx"),
        )
        .unwrap();
        let vector = |key: &str, detector: &mut dyn rustface::Detector| {
            let faces = face_crops(detector, &std::env::var(key).unwrap()).unwrap();
            assert_eq!(
                faces.len(),
                1,
                "{key} must contain exactly one detected face"
            );
            embedding(&model, &faces[0]).unwrap()
        };
        let a = vector("CHEKI_FACE_A", detector.as_mut());
        let b = vector("CHEKI_FACE_B", detector.as_mut());
        let c = vector("CHEKI_FACE_C", detector.as_mut());
        eprintln!("same={} different={} ", cosine(&a, &b), cosine(&a, &c));
    }
}
