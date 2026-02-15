mod cmds;
mod config;
mod error;
mod scope;
mod state;

use error::*;
use scope::*;
use state::*;


/// Initializes the plugin.
pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R, Option<config::Config>> {
	tauri::plugin::Builder::<R, Option<config::Config>>::new("fs-stream")
		.setup(|app, api| {
			use tauri::Manager as _;

			let require_literal_leading_dot = api
				.config()
				.as_ref()
                .and_then(|c| c.require_literal_leading_dot);

			app.manage(new_plugin_config_state(require_literal_leading_dot));
			app.manage(new_plugin_resources_state(app.app_handle().clone()));
			Ok(())
		})
		.invoke_handler(tauri::generate_handler![
			cmds::open_read_file_stream,
			cmds::open_read_text_file_lines_stream,
			cmds::open_write_file_stream,
			cmds::close_all_file_streams,
		])
		.js_init_script(format!(
            "window.__TAURI_FS_STREAM_PLUGIN_INTERNALS__ = {{ supportsRawIpcRequestBody: {} }};",
            // https://github.com/tauri-apps/tauri/issues/10573
			cfg!(not(any(target_os = "android", target_os = "linux")))
        ))
		.build()
}