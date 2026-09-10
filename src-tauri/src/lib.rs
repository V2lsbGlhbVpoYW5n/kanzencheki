mod storage;
use std::sync::{Arc, Mutex};
use storage::{Crop, ImportOptions, ImportReport, Library, Metadata, Store};
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
#[derive(Clone)]
struct Backend(Arc<Mutex<Store>>);
async fn work<T: Send + 'static>(
    backend: Backend,
    f: impl FnOnce(&mut Store) -> anyhow::Result<T> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = backend.0.lock().map_err(|_| "图库锁不可用".to_string())?;
        f(&mut store).map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn library_load(state: State<'_, Backend>) -> Result<Library, String> {
    work(state.inner().clone(), |s| s.list()).await
}
#[tauri::command]
async fn cheki_update(
    state: State<'_, Backend>,
    id: String,
    metadata: Metadata,
) -> Result<Library, String> {
    work(state.inner().clone(), move |s| s.update(&id, metadata)).await
}
#[tauri::command]
async fn asset_link(
    state: State<'_, Backend>,
    cheki_id: String,
    asset_id: String,
) -> Result<Library, String> {
    work(state.inner().clone(), move |s| s.link(&cheki_id, &asset_id)).await
}
#[tauri::command]
async fn preview_retry(state: State<'_, Backend>, asset_id: String) -> Result<Library, String> {
    work(state.inner().clone(), move |s| {
        s.refresh_preview(&asset_id)?;
        s.list()
    })
    .await
}
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    id: String,
    title: String,
    done: usize,
    total: usize,
    state: String,
    detail: String,
}
fn progress(
    app: &tauri::AppHandle,
    id: &str,
    title: &str,
    done: usize,
    total: usize,
    state: &str,
    detail: &str,
) {
    let _ = app.emit(
        "library-progress",
        Progress {
            id: id.into(),
            title: title.into(),
            done,
            total,
            state: state.into(),
            detail: detail.into(),
        },
    );
}
#[tauri::command]
async fn import_photos(
    app: tauri::AppHandle,
    state: State<'_, Backend>,
    options: ImportOptions,
    task_id: String,
    location_id: Option<String>,
) -> Result<Option<ImportReport>, String> {
    let paths = if let Some(location) = location_id {
        work(state.inner().clone(), move |s| s.location_files(&location)).await?
    } else {
        let dialog_app = app.clone();
        let paths = tauri::async_runtime::spawn_blocking(move || {
            dialog_app
                .dialog()
                .file()
                .add_filter("拍立得影像", &["jpg", "jpeg", "png", "webp", "tif", "tiff"])
                .blocking_pick_files()
        })
        .await
        .map_err(|e| e.to_string())?;
        let Some(paths) = paths else { return Ok(None) };
        paths
            .into_iter()
            .map(|p| p.into_path().map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?
    };
    if !["", "each", "collection", "versions"].contains(&options.grouping.as_str()) {
        return Err("无效的导入组织方式".into());
    }
    let mut attach_cheki = options.cheki_id.clone();
    let mut attach_asset = options.asset_id.clone();
    let total = paths.len();
    let mut imported = 0;
    let mut errors = vec![];
    for (i, path) in paths.into_iter().enumerate() {
        progress(
            &app,
            &task_id,
            "导入影像",
            i,
            total,
            "running",
            &path.file_name().unwrap_or_default().to_string_lossy(),
        );
        let opts = ImportOptions {
            cheki_id: attach_cheki.clone(),
            asset_id: attach_asset.clone(),
            reference: options.reference,
            kind: options.kind.clone(),
            grouping: options.grouping.clone(),
        };
        match work(state.inner().clone(), move |s| {
            s.import_file(&path, &opts)?;
            s.latest_link()
        })
        .await
        {
            Ok((cheki, asset)) => {
                imported += 1;
                if options.grouping == "collection" || options.grouping == "versions" {
                    if attach_cheki.is_none() {
                        attach_cheki = Some(cheki);
                    }
                    if options.grouping == "versions" && attach_asset.is_none() {
                        attach_asset = Some(asset);
                    }
                }
            }
            Err(e) => errors.push(e),
        }
    }
    progress(
        &app,
        &task_id,
        "导入影像",
        total,
        total,
        if errors.is_empty() { "done" } else { "error" },
        &format!(
            "已导入 {imported} 份影像{}",
            if errors.is_empty() {
                String::new()
            } else {
                format!("；{}", errors.join("；"))
            }
        ),
    );
    let library = work(state.inner().clone(), |s| s.list()).await?;
    Ok(Some(ImportReport {
        imported,
        errors,
        library,
    }))
}
#[tauri::command]
async fn location_add(
    app: tauri::AppHandle,
    state: State<'_, Backend>,
    replace: Option<String>,
) -> Result<Option<Library>, String> {
    let path =
        tauri::async_runtime::spawn_blocking(move || app.dialog().file().blocking_pick_folder())
            .await
            .map_err(|e| e.to_string())?;
    let Some(path) = path else { return Ok(None) };
    let path = path.into_path().map_err(|e| e.to_string())?;
    work(state.inner().clone(), move |s| {
        s.add_location(&path, replace.as_deref())
    })
    .await
    .map(Some)
}
#[tauri::command]
async fn cheki_trash(
    state: State<'_, Backend>,
    ids: Vec<String>,
    restore: bool,
) -> Result<Library, String> {
    work(state.inner().clone(), move |s| s.trash(&ids, restore)).await
}
#[tauri::command]
async fn cheki_cover(
    state: State<'_, Backend>,
    cheki_id: String,
    asset_id: Option<String>,
) -> Result<Library, String> {
    work(state.inner().clone(), move |s| s.cover(&cheki_id, asset_id)).await
}
#[tauri::command]
async fn cheki_merge(
    state: State<'_, Backend>,
    target: String,
    source: String,
) -> Result<Library, String> {
    work(state.inner().clone(), move |s| {
        s.merge_chekis(&target, &source)
    })
    .await
}
#[tauri::command]
async fn asset_detach(
    state: State<'_, Backend>,
    cheki_id: String,
    asset_id: String,
) -> Result<Library, String> {
    work(state.inner().clone(), move |s| {
        s.detach(&cheki_id, &asset_id)
    })
    .await
}
#[tauri::command]
async fn asset_kind(
    state: State<'_, Backend>,
    asset_id: String,
    kind: String,
) -> Result<Library, String> {
    work(state.inner().clone(), move |s| {
        s.asset_kind(&asset_id, &kind)
    })
    .await
}
#[tauri::command]
async fn rendition_prefer(
    state: State<'_, Backend>,
    asset_id: String,
    rendition_id: Option<String>,
) -> Result<Library, String> {
    work(state.inner().clone(), move |s| {
        s.prefer_source(&asset_id, rendition_id)
    })
    .await
}
#[tauri::command]
async fn asset_crop(
    state: State<'_, Backend>,
    asset_id: String,
    crop: Option<Crop>,
) -> Result<Library, String> {
    work(state.inner().clone(), move |s| s.crop(&asset_id, crop)).await
}
#[tauri::command]
async fn crop_suggest(state: State<'_, Backend>, asset_id: String) -> Result<Crop, String> {
    work(state.inner().clone(), move |s| s.auto_crop(&asset_id)).await
}
#[tauri::command]
async fn cache_resume(
    app: tauri::AppHandle,
    state: State<'_, Backend>,
    task_id: String,
) -> Result<Library, String> {
    let pending = work(state.inner().clone(), |s| s.pending()).await?;
    let total = pending.len();
    let mut errors = vec![];
    for (i, asset) in pending.into_iter().enumerate() {
        progress(
            &app,
            &task_id,
            "生成离线浏览图",
            i,
            total,
            "running",
            &asset,
        );
        if let Err(e) = work(state.inner().clone(), move |s| s.refresh_preview(&asset)).await {
            errors.push(e);
        }
    }
    progress(
        &app,
        &task_id,
        "生成离线浏览图",
        total,
        total,
        if errors.is_empty() { "done" } else { "error" },
        &if errors.is_empty() {
            "缓存已就绪".into()
        } else {
            errors.join("；")
        },
    );
    work(state.inner().clone(), |s| s.list()).await
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let root = std::env::var_os("CHEKI_LIBRARY_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?.join("library"));
            let store = Store::open(root)?;
            app.asset_protocol_scope()
                .allow_directory(store.root.join("previews"), true)?;
            app.manage(Backend(Arc::new(Mutex::new(store))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            library_load,
            cheki_update,
            import_photos,
            asset_link,
            preview_retry,
            location_add,
            cheki_trash,
            cheki_cover,
            cheki_merge,
            asset_detach,
            asset_kind,
            asset_crop,
            crop_suggest,
            cache_resume,
            rendition_prefer
        ])
        .run(tauri::generate_context!())
        .expect("Could not start Cheki Gallery");
}
