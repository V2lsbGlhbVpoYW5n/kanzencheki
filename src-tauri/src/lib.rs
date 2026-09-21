mod detection;
mod media;
mod storage;
use std::sync::{Arc, Mutex};
use storage::{
    Crop, DocumentContent, DocumentDraft, ImageView, ImportOptions, ImportReport, Library,
    Metadata, Person, PersonDraft, PersonSpace, Store,
};
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
    work(state.inner().clone(), |s| {
        s.recover_rotations()?;
        s.list()
    })
    .await
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
async fn detection_confirm(state: State<'_, Backend>, id: String) -> Result<Library, String> {
    work(state.inner().clone(), move |s| s.confirm_detection(&id)).await
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
    literal_detail: bool,
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
            literal_detail: state == "running" && (title == "导入影像" || title == "导入人物附件"),
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
    locale: Option<String>,
) -> Result<Option<ImportReport>, String> {
    let paths = if let Some(location) = location_id {
        work(state.inner().clone(), move |s| s.location_files(&location)).await?
    } else {
        let dialog_app = app.clone();
        let paths = tauri::async_runtime::spawn_blocking(move || {
            dialog_app
                .dialog()
                .file()
                .add_filter(
                    match locale.as_deref() {
                        Some("en") => "Cheki images",
                        Some("ja") => "チェキ画像",
                        _ => "拍立得影像",
                    },
                    &["jpg", "jpeg", "png", "webp", "tif", "tiff"],
                )
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
    if !["", "each", "collection"].contains(&options.grouping.as_str()) {
        return Err("无效的导入组织方式".into());
    }
    let mut attach_cheki = options.cheki_id.clone();
    let total = paths.len();
    let mut imported = 0;
    let mut detection_ids = Vec::new();
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
            reference: options.reference,
            grouping: options.grouping.clone(),
        };
        match work(state.inner().clone(), move |s| {
            s.import_file(&path, &opts)?;
            s.latest_link()
        })
        .await
        {
            Ok((cheki, _asset)) => {
                imported += 1;
                if !detection_ids.contains(&cheki) {
                    detection_ids.push(cheki.clone());
                }
                if options.grouping == "collection" {
                    if attach_cheki.is_none() {
                        attach_cheki = Some(cheki);
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
    let jobs = work(state.inner().clone(), move |s| {
        s.detection_jobs(&detection_ids)
    })
    .await?;
    let references = work(state.inner().clone(), |s| s.detection_references()).await?;
    let library = work(state.inner().clone(), |s| s.list()).await?;
    detection::start(app, state.inner().clone(), jobs, references);
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
    sources: Vec<String>,
) -> Result<Library, String> {
    work(state.inner().clone(), move |s| {
        s.merge_many(&target, &sources)
    })
    .await
}
#[tauri::command]
async fn asset_rotate(state: State<'_, Backend>, asset_id: String) -> Result<Library, String> {
    work(state.inner().clone(), move |s| s.rotate(&asset_id)).await
}
#[tauri::command]
async fn crop_preview(
    state: State<'_, Backend>,
    asset_id: String,
    crop: Crop,
) -> Result<Vec<u8>, String> {
    work(state.inner().clone(), move |s| {
        s.crop_preview(&asset_id, crop)
    })
    .await
}
#[tauri::command]
async fn asset_view(
    app: tauri::AppHandle,
    state: State<'_, Backend>,
    asset_id: String,
) -> Result<ImageView, String> {
    let view = work(state.inner().clone(), move |s| s.image_view(&asset_id)).await?;
    app.asset_protocol_scope()
        .allow_file(&view.path)
        .map_err(|e| e.to_string())?;
    Ok(view)
}
#[tauri::command]
async fn asset_view_release(state: State<'_, Backend>, path: String) -> Result<(), String> {
    work(state.inner().clone(), move |s| s.release_view(&path)).await
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
async fn cheki_purge(state: State<'_, Backend>, ids: Vec<String>) -> Result<Library, String> {
    work(state.inner().clone(), move |s| s.purge(&ids)).await
}
#[tauri::command]
async fn asset_delete(
    state: State<'_, Backend>,
    cheki_id: String,
    asset_id: String,
) -> Result<Library, String> {
    work(state.inner().clone(), move |s| {
        s.delete_asset(&cheki_id, &asset_id)
    })
    .await
}
#[tauri::command]
async fn location_remove(
    state: State<'_, Backend>,
    location_id: String,
) -> Result<Library, String> {
    work(state.inner().clone(), move |s| {
        s.remove_location(&location_id)
    })
    .await
}
#[tauri::command]
async fn library_clear_local(
    state: State<'_, Backend>,
    confirmation: String,
) -> Result<Library, String> {
    work(state.inner().clone(), move |s| s.clear_local(&confirmation)).await
}
#[tauri::command]
async fn person_save(state: State<'_, Backend>, draft: PersonDraft) -> Result<Person, String> {
    work(state.inner().clone(), move |s| s.save_person(draft)).await
}
#[tauri::command]
async fn person_trash(
    state: State<'_, Backend>,
    person_id: String,
    restore: bool,
) -> Result<Library, String> {
    work(state.inner().clone(), move |s| {
        s.person_trash(&person_id, restore)
    })
    .await
}
#[tauri::command]
async fn person_purge(state: State<'_, Backend>, person_id: String) -> Result<Library, String> {
    work(state.inner().clone(), move |s| s.person_purge(&person_id)).await
}
#[tauri::command]
async fn person_space(state: State<'_, Backend>, person_id: String) -> Result<PersonSpace, String> {
    work(state.inner().clone(), move |s| s.person_space(&person_id)).await
}
#[tauri::command]
async fn person_file_rename(
    state: State<'_, Backend>,
    person_id: String,
    file_id: String,
    name: String,
) -> Result<PersonSpace, String> {
    work(state.inner().clone(), move |s| {
        s.rename_person_file(&person_id, &file_id, &name)
    })
    .await
}
#[tauri::command]
async fn person_file_delete(
    state: State<'_, Backend>,
    person_id: String,
    file_id: String,
) -> Result<PersonSpace, String> {
    work(state.inner().clone(), move |s| {
        s.delete_person_file(&person_id, &file_id)
    })
    .await
}
#[tauri::command]
async fn person_document_read(
    state: State<'_, Backend>,
    person_id: String,
    document_id: String,
) -> Result<DocumentContent, String> {
    work(state.inner().clone(), move |s| {
        s.read_document(&person_id, &document_id)
    })
    .await
}
#[tauri::command]
async fn person_document_save(
    state: State<'_, Backend>,
    draft: DocumentDraft,
) -> Result<PersonSpace, String> {
    work(state.inner().clone(), move |s| s.save_document(draft)).await
}
#[tauri::command]
async fn person_document_delete(
    state: State<'_, Backend>,
    person_id: String,
    document_id: String,
) -> Result<PersonSpace, String> {
    work(state.inner().clone(), move |s| {
        s.delete_document(&person_id, &document_id)
    })
    .await
}
#[tauri::command]
async fn person_video_cover(
    state: State<'_, Backend>,
    person_id: String,
    file_id: String,
) -> Result<Vec<u8>, String> {
    let file = work(state.inner().clone(), move |s| {
        s.person_space(&person_id)?
            .files
            .into_iter()
            .find(|f| f.id == file_id && f.available && f.mime_type.starts_with("video/"))
            .ok_or_else(|| anyhow::anyhow!("视频不可用"))
    })
    .await?;
    tauri::async_runtime::spawn_blocking(move || {
        media::first_frame(std::path::Path::new(&file.src)).map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn person_media_open(
    state: State<'_, Backend>,
    person_id: String,
    file_id: String,
) -> Result<(), String> {
    let file = work(state.inner().clone(), move |s| {
        s.person_space(&person_id)?
            .files
            .into_iter()
            .find(|f| {
                f.id == file_id
                    && f.available
                    && (f.mime_type.starts_with("video/") || f.mime_type.starts_with("audio/"))
            })
            .ok_or_else(|| anyhow::anyhow!("音视频文件不可用"))
    })
    .await?;
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&file.src)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = file;
        Err("此平台请使用内嵌播放".into())
    }
}
#[tauri::command]
async fn person_file_export(
    app: tauri::AppHandle,
    state: State<'_, Backend>,
    person_id: String,
    file_id: String,
) -> Result<(), String> {
    let file = work(state.inner().clone(), move |s| {
        s.person_space(&person_id)?
            .files
            .into_iter()
            .find(|f| f.id == file_id)
            .ok_or_else(|| anyhow::anyhow!("附件不存在"))
    })
    .await?;
    let name = file.filename.clone();
    let destination = tauri::async_runtime::spawn_blocking(move || {
        app.dialog().file().set_file_name(name).blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    if let Some(dest) = destination {
        let dest = dest.into_path().map_err(|e| e.to_string())?;
        tauri::async_runtime::spawn_blocking(move || std::fs::copy(file.src, dest))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
async fn person_files_import(
    app: tauri::AppHandle,
    state: State<'_, Backend>,
    person_id: String,
    task_id: String,
) -> Result<PersonSpace, String> {
    let dialog = app.clone();
    let paths =
        tauri::async_runtime::spawn_blocking(move || dialog.dialog().file().blocking_pick_files())
            .await
            .map_err(|e| e.to_string())?;
    if let Some(paths) = paths {
        let total = paths.len();
        let mut errors = vec![];
        for (i, path) in paths.into_iter().enumerate() {
            let path = path.into_path().map_err(|e| e.to_string())?;
            let person = person_id.clone();
            progress(
                &app,
                &task_id,
                "导入人物附件",
                i,
                total,
                "running",
                &path.file_name().unwrap_or_default().to_string_lossy(),
            );
            if let Err(e) = work(state.inner().clone(), move |s| {
                s.import_person_file(&person, &path)
            })
            .await
            {
                errors.push(e);
            }
        }
        progress(
            &app,
            &task_id,
            "导入人物附件",
            total,
            total,
            if errors.is_empty() { "done" } else { "error" },
            &if errors.is_empty() {
                format!("已导入 {total} 个附件")
            } else {
                errors.join("；")
            },
        );
    } else {
        progress(&app, &task_id, "导入人物附件", 0, 0, "done", "已取消");
    }
    work(state.inner().clone(), move |s| s.person_space(&person_id)).await
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
            app.asset_protocol_scope()
                .allow_directory(store.root.join("people"), true)?;
            app.manage(Backend(Arc::new(Mutex::new(store))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            library_load,
            detection_confirm,
            person_save,
            person_trash,
            person_purge,
            person_space,
            person_file_rename,
            person_file_export,
            person_video_cover,
            person_media_open,
            person_files_import,
            person_file_delete,
            person_document_read,
            person_document_save,
            person_document_delete,
            cheki_purge,
            asset_delete,
            library_clear_local,
            cheki_update,
            import_photos,
            location_add,
            location_remove,
            cheki_trash,
            cheki_cover,
            cheki_merge,
            asset_crop,
            asset_rotate,
            crop_preview,
            asset_view,
            asset_view_release,
            crop_suggest,
        ])
        .run(tauri::generate_context!())
        .expect("Could not start Cheki Gallery");
}
