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



pub type PluginResourcesState<'a, R> = tauri::State<'a, PluginResourcesStateInner<R>>;
pub type PluginResourcesStateInner<R> = std::sync::Arc::<PluginResources<R>>;

pub fn new_plugin_resources_state<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> PluginResourcesStateInner<R> {
    std::sync::Arc::new(PluginResources::new(app))
}

pub struct PluginResources<R: tauri::Runtime> {
    list: std::sync::Mutex<std::collections::HashSet<tauri::ResourceId>>,
    app: tauri::AppHandle<R>,
}

impl<R: tauri::Runtime> PluginResources<R> {

    fn new(app: tauri::AppHandle<R>) -> Self {
        Self {
            list: std::sync::Mutex::new(std::collections::HashSet::new()),
            app
        }
    }

    pub fn add(&self, r: impl tauri::Resource) -> Result<tauri::ResourceId> {
        let id = self.app.resources_table().add(r);
        self.list.lock()?.insert(id);
        Ok(id)
    }

    pub fn get<T: tauri::Resource>(&self, id: tauri::ResourceId) -> Result<std::sync::Arc<T>> {
        Ok(self.app.resources_table().get(id)?)
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
        let ids: Vec<_> = {
            let mut locked = self.list.lock()?;
            let ids = locked.iter().copied().collect();
            locked.clear();
            ids
        };

        let mut rt = self.app.resources_table();
        for id in ids {
            if rt.has(id) {
                rt.close(id)?;
            }
        }

        Ok(())
    }
}