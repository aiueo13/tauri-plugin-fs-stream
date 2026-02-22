use crate::*;
use tauri::Manager as _;


pub type PluginConfigState<'a> = tauri::State<'a, PluginConfigStateInner>;
pub type PluginConfigStateInner = std::sync::Arc<PluginConfig>;

pub fn new_plugin_config_state(require_literal_leading_dot: Option<bool>) -> PluginConfigStateInner {
    std::sync::Arc::new(PluginConfig { require_literal_leading_dot })
}

pub struct PluginConfig {
    pub require_literal_leading_dot: Option<bool>
}



pub type PluginFileResourcesState<'a, R> = tauri::State<'a, PluginFileResourcesStateInner<R>>;
pub type PluginFileResourcesStateInner<R> = std::sync::Arc::<PluginFileResources<R>>;

pub fn new_plugin_file_resources_state<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> PluginFileResourcesStateInner<R> {
    std::sync::Arc::new(PluginFileResources::new(app))
}

pub struct PluginFileResources<R: tauri::Runtime> {
    list: std::sync::Mutex<std::collections::HashSet<tauri::ResourceId>>,
    app: tauri::AppHandle<R>,
}

impl<R: tauri::Runtime> PluginFileResources<R> {

    fn new(app: tauri::AppHandle<R>) -> Self {
        Self {
            list: std::sync::Mutex::new(std::collections::HashSet::new()),
            app
        }
    }

    pub fn add<T: Sync + Send + 'static>(&self, r: T) -> Result<tauri::ResourceId> {
        let id = self.app.resources_table().add(PluginResource::new(r));
        self.list.lock()?.insert(id);
        Ok(id)
    }

    pub fn get<T: Sync + Send + 'static>(&self, id: tauri::ResourceId) -> Result<std::sync::Arc<T>> {
        let r = self.app.resources_table().get::<PluginResource<T>>(id)?;
        Ok(std::sync::Arc::clone(&r.resource))
    }

    pub fn close(&self, id: tauri::ResourceId) -> Result<()> {  
        self.list.lock()?.remove(&id);
        
        let mut rt = self.app.resources_table();
        if rt.has(id) {
            rt.close(id)?;
        }

        Ok(())
    }

    pub fn close_all(&self) -> Result<()> {
        let ids = self.list.lock()?
            .drain()
            .collect::<Vec<tauri::ResourceId>>();

        let mut rt = self.app.resources_table();
        for id in ids {
            if rt.has(id) {
                rt.close(id)?;
            }
        }

        Ok(())
    }

    pub fn count(&self) -> Result<usize> {
        let ids = self.list.lock()?
            .iter()
            .cloned()
            .collect::<Vec<tauri::ResourceId>>();

        let mut count = 0;
        let rt = self.app.resources_table();
        for id in ids {
            if rt.has(id) {
                count += 1;
            }
        }

        Ok(count)
    }
}

struct PluginResource<T> {
    resource: std::sync::Arc<T>
}

impl<T> PluginResource<T> {

    fn new(resource: T) -> Self {
        Self { resource: std::sync::Arc::new(resource) }
    }
}

impl<T: Sync + Send + 'static> tauri::Resource for PluginResource<T> {}