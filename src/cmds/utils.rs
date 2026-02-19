use crate::*;
use tauri::Manager as _;
use tauri_plugin_fs::FsExt as _;


pub fn convert_rid_to_bytes(rid: tauri::ResourceId) -> Vec<u8> {
    let rid: u32 = rid;
    rid.to_be_bytes().to_vec()
}

// Based on code from tauri-plugin-fs crate
//
// Source:
// - https://github.com/tauri-apps/plugins-workspace/blob/6c3da6d2901590eff4daa2d84a21a6883085776f/plugins/fs/src/commands.rs#L1129
// - Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// - Licensed under the MIT License or the Apache 2.0 License
pub fn resolve_path<R: tauri::Runtime>(
    webview: &tauri::Webview<R>,
    global_scope: &tauri::ipc::GlobalScope<Scope>,
    command_scope: &tauri::ipc::CommandScope<Scope>,
    config: &PluginConfig,
    path: tauri_plugin_fs::SafeFilePath,
    base_dir: Option<tauri::path::BaseDirectory>,
) -> Result<std::path::PathBuf> {

    let path = path.into_path()?;
    let path = if let Some(base_dir) = base_dir {
        webview.path().resolve(&path, base_dir)?
    } else {
        path.to_path_buf()
    };

    let fs_scope = webview.try_fs_scope();

    let scope = tauri::scope::fs::Scope::new(
        webview,
        &tauri::utils::config::FsScope::Scope {
            allow: global_scope
                .allows()
                .iter()
                .filter_map(|e| e.path.clone())
                .chain(command_scope.allows().iter().filter_map(|e| e.path.clone()))
                .collect(),
            deny: global_scope
                .denies()
                .iter()
                .filter_map(|e| e.path.clone())
                .chain(command_scope.denies().iter().filter_map(|e| e.path.clone()))
                .collect(),
            require_literal_leading_dot: config.require_literal_leading_dot,
        },
    )?;

    let require_literal_leading_dot = config.require_literal_leading_dot.unwrap_or(cfg!(unix));

    if is_forbidden(&scope, &path, require_literal_leading_dot) {
        return Err(Error::with(format!("forbidden path: {}", path.display())));
    }
    if fs_scope.as_ref().is_some_and(|s| is_forbidden(&s, &path, require_literal_leading_dot)) {
        return Err(Error::with(format!("forbidden path: {}", path.display())));
    }

    if scope.is_allowed(&path) {
        return Ok(path)
    }
    if fs_scope.as_ref().is_some_and(|s| s.is_allowed(&path)) {
        return Ok(path)
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
// - https://github.com/tauri-apps/plugins-workspace/blob/6c3da6d2901590eff4daa2d84a21a6883085776f/plugins/fs/src/commands.rs#L1190
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
    } else {
        path.to_path_buf()
    };
    let path = if !path.exists() {
        crate::Result::Ok(path)
    } else {
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
    } else {
        false
    }
}

// Based on code from tauri-plugin-fs crate
//
// Source:
// - https://github.com/tauri-apps/plugins-workspace/blob/3d0d2e041bbad9766aebecaeba291a28d8d7bf5c/plugins/fs/src/lib.rs#L347
// - Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// - Licensed under the MIT License or the Apache 2.0 License
impl tauri::ipc::ScopeObject for Scope {
    type Error = Error;

    fn deserialize<R: tauri::Runtime>(
        app: &tauri::AppHandle<R>,
        raw: tauri::utils::acl::Value,
    ) -> Result<Self> {
        let path = serde_json::from_value(raw.into()).map(|raw| match raw {
            ScopeSchema::Value(path) => path,
            ScopeSchema::Object { path } => path,
        })?;

        use tauri::Manager as _;

        match app.path().parse(path) {
            Ok(path) => Ok(Self { path: Some(path) }),
            Err(err) => Err(err.into()),
        }
    }
}
