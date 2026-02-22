use crate::*;


#[tauri::command]
pub async fn count_all_file_streams<R: tauri::Runtime>(
    resources: PluginFileResourcesState<'_, R>,
    _app: tauri::AppHandle<R>,
) -> Result<usize> {

    let resources = std::sync::Arc::clone(&resources);

    tauri::async_runtime::spawn_blocking(move || {
        let count = resources.count()?;
        Ok(count)
    }).await?
}