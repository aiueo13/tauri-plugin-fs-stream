use crate::*;


#[tauri::command]
pub async fn close_all_file_streams<R: tauri::Runtime>(
    resources: PluginResourcesState<'_, R>,
    _app: tauri::AppHandle<R>,
) -> Result<()> {

    let resources = std::sync::Arc::clone(&resources);

    tauri::async_runtime::spawn_blocking(move || {
        resources.close_all()?;
        Ok(())
    }).await?
}