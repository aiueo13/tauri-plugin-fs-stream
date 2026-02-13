mod cmds;
mod error;

pub use error::{Error, Result};


/// Initializes the plugin.
pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
	tauri::plugin::Builder::new("fs-stream")
		.invoke_handler(tauri::generate_handler![
			cmds::open_read_file_stream,
			cmds::open_read_text_file_lines_stream,
			cmds::open_write_file_stream,
		])
		.js_init_script(format!(
            "window.__TAURI_FS_STREAM_PLUGIN_INTERNALS__ = {{ supportsRawIpcRequestBody: {} }};",
            // https://github.com/tauri-apps/tauri/issues/10573
			cfg!(not(any(target_os = "android", target_os = "linux")))
        ))
		.build()
}