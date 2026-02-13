use super::*;
use crate::*;


pub fn convert_rid_to_bytes(rid: tauri::ResourceId) -> Vec<u8> {
    let rid: u32 = rid;
    rid.to_be_bytes().to_vec()
}

pub struct PluginResource<T> {
    resource: std::sync::Arc<T>
}

impl<T> PluginResource<T> {

    pub fn new(resource: T) -> Self {
        Self { resource: std::sync::Arc::new(resource) }
    }

    pub fn get(&self) -> std::sync::Arc<T> {
        std::sync::Arc::clone(&self.resource)
    }
}

impl<T: Sync + Send + 'static> tauri::Resource for PluginResource<T> {}

// Based on code from tauri-plugin-fs crate
//
// Source:
// - https://github.com/tauri-apps/plugins-workspace/blob/3d0d2e041bbad9766aebecaeba291a28d8d7bf5c/plugins/fs/src/commands.rs#L1090
// - Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// - Licensed under the MIT License or the Apache 2.0 License
#[must_use]
pub fn validate_path_permission<R: tauri::Runtime>(
    path: impl AsRef<std::path::Path>,
    app: &tauri::AppHandle<R>,
    cmd_scope: &tauri::ipc::CommandScope<scope::Scope>,
    global_scope: &tauri::ipc::GlobalScope<scope::Scope>,
) -> Result<()> {

    let path = path.as_ref();
    let require_literal_leading_dot = true;

    let scope = tauri::scope::fs::Scope::new(
        app,
        &tauri::utils::config::FsScope::Scope {
            allow: global_scope
                .allows()
                .iter()
                .filter_map(|e| e.path.clone())
                .chain(cmd_scope.allows().iter().filter_map(|e| e.path.clone()))
                .collect(),

            deny: global_scope
                .denies()
                .iter()
                .filter_map(|e| e.path.clone())
                .chain(cmd_scope.denies().iter().filter_map(|e| e.path.clone()))
                .collect(),

            require_literal_leading_dot: Some(require_literal_leading_dot),
        },
    )?;

    if !is_forbidden(&scope, &path, require_literal_leading_dot) && scope.is_allowed(&path) {
        return Ok(());
    }

    if cfg!(debug_assertions) {
        Err(Error::with(format!(
            "forbidden path: {}, maybe it is not allowed on the scope configuration in your capability file",
            path.display()
        )))
    }
    else {
        Err(Error::with(format!("forbidden path: {}", path.display())))
    }
}

// Based on code from tauri-plugin-fs crate
//
// Source:
// - https://github.com/tauri-apps/plugins-workspace/blob/3d0d2e041bbad9766aebecaeba291a28d8d7bf5c/plugins/fs/src/commands.rs#L1151
// - Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// - Licensed under the MIT License or the Apache 2.0 License
fn is_forbidden<P: AsRef<std::path::Path>>(
    scope: &tauri::fs::Scope,
    path: P,
    require_literal_leading_dot: bool,
) -> bool {

    let path = path.as_ref();
    let path = if path.is_symlink() {
        match std::fs::read_link(path) {
            Ok(p) => p,
            Err(_) => return false,
        }
    }
    else {
        path.to_path_buf()
    };

    let path = if !path.exists() {
        crate::Result::Ok(path)
    }
    else {
        std::fs::canonicalize(path).map_err(Into::into)
    };

    if let Ok(path) = path {
        let path: std::path::PathBuf = path.components().collect();
        scope.forbidden_patterns().iter().any(|p| {
            p.matches_path_with(
                &path,
                glob::MatchOptions {
                    // this is needed so `/dir/*` doesn't match files within subdirectories such as `/dir/subdir/file.txt`
                    // see: <https://github.com/tauri-apps/tauri/security/advisories/GHSA-6mv3-wm7j-h4w5>
                    require_literal_separator: true,
                    require_literal_leading_dot,
                    ..Default::default()
                },
            )
        })
    } 
    else {
        false
    }
}

// Based on code from tauri-plugin-fs crate
//
// Source:
// - https://github.com/tauri-apps/plugins-workspace/blob/3d0d2e041bbad9766aebecaeba291a28d8d7bf5c/plugins/fs/src/lib.rs#L347
// - Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// - Licensed under the MIT License or the Apache 2.0 License
impl tauri::ipc::ScopeObject for scope::Scope {
    type Error = Error;

    fn deserialize<R: tauri::Runtime>(
        app: &tauri::AppHandle<R>,
        raw: tauri::utils::acl::Value,
    ) -> Result<Self> {
        let path = serde_json::from_value(raw.into()).map(|raw| match raw {
            scope::ScopeSchema::Value(path) => path,
            scope::ScopeSchema::Object { path } => path,
        })?;

        use tauri::Manager as _;

        match app.path().parse(path) {
            Ok(path) => Ok(Self { path: Some(path) }),
            Err(err) => Err(err.into()),
        }
    }
}
