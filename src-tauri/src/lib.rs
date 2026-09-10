mod storage;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use storage::{Store, Metadata, Library, ImportReport};
#[derive(Clone)]
struct Backend(Arc<Mutex<Store>>);
async fn work<T: Send + 'static>(backend: Backend, f: impl FnOnce(&mut Store) -> anyhow::Result<T> + Send + 'static) -> Result<T,String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = backend.0.lock().map_err(|_| "图库锁不可用".to_string())?;
        f(&mut store).map_err(|e|format!("{e:#}"))
    }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn library_load(state: State<'_, Backend>) -> Result<Library,String> {work(state.inner().clone(),|s|s.list()).await}
#[tauri::command]
async fn cheki_update(state: State<'_, Backend>, id: String, metadata: Metadata) -> Result<Library,String> {work(state.inner().clone(),move |s|s.update(&id,metadata)).await}
#[tauri::command]
async fn asset_link(state: State<'_, Backend>, cheki_id: String, asset_id: String) -> Result<Library,String> {work(state.inner().clone(),move |s|s.link(&cheki_id,&asset_id)).await}
#[tauri::command]
async fn preview_retry(state: State<'_, Backend>, asset_id: String) -> Result<Library,String> {work(state.inner().clone(),move |s|s.retry_preview(&asset_id)).await}
#[tauri::command]
async fn import_photos(app: tauri::AppHandle, state: State<'_, Backend>, cheki_id: Option<String>) -> Result<Option<ImportReport>,String> {
    let paths = tauri::async_runtime::spawn_blocking(move || app.dialog().file().add_filter("照片", &["jpg","jpeg","png","webp","tif","tiff"]).blocking_pick_files()).await.map_err(|e|e.to_string())?;
    let Some(paths) = paths else {return Ok(None)};
    let paths = paths.into_iter().map(|p|p.into_path().map_err(|e|e.to_string())).collect::<Result<Vec<_>,_>>()?;
    work(state.inner().clone(),move |s|s.import(paths,cheki_id)).await.map(Some)
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let root = std::env::var_os("CHEKI_LIBRARY_DIR").map(std::path::PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?.join("library"));
            let store = Store::open(root)?;
            app.asset_protocol_scope().allow_directory(store.root.join("previews"), true)?;
            app.manage(Backend(Arc::new(Mutex::new(store))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![library_load,cheki_update,import_photos,asset_link,preview_retry])
        .run(tauri::generate_context!())
        .expect("Could not start Cheki Gallery");
}
