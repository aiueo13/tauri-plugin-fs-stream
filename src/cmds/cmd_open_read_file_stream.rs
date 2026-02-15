use crate::*;
use super::*;
use std::io::Read as _;


#[tauri::command]
pub async fn open_read_file_stream<R: tauri::Runtime>(
    event: OpenReadFileStreamEventInput,
    webview: tauri::Webview<R>,
    cmd_scope: tauri::ipc::CommandScope<Scope>,
    global_scope: tauri::ipc::GlobalScope<Scope>,
    resources: PluginResourcesState<'_, R>,
    config: PluginConfigState<'_>,
) -> Result<tauri::ipc::Response> {

    type FileResource = PluginResource<std::sync::Mutex<FileResourceInner>>;

    
    let resources = std::sync::Arc::clone(&resources);
    let config = std::sync::Arc::clone(&config);
    
    match event {
        OpenReadFileStreamEventInput::Open { path, base_dir } => {
            let path = resolve_path(
                &webview,
                &global_scope, 
                &cmd_scope,
                &config,
                path,
                base_dir
            )?;
            
            tauri::async_runtime::spawn_blocking(move || {
                let file = std::fs::File::open(&path)?;
                let len = file.metadata()?.len();
                let res = FileResourceInner {
                    file, 
                    init_file_len: len,
                    read: 0
                };
                let id = resources.add(FileResource::new(std::sync::Mutex::new(res)))?;
                OpenReadFileStreamEventOutput::Open(id).try_into()
            }).await?
        },
        OpenReadFileStreamEventInput::Read { id, len } => {
            tauri::async_runtime::spawn_blocking(move || -> Result<_> {
                let state = resources.get::<FileResource>(id)?.get();
                let mut state = state.lock()?;
                
                if state.init_file_len <= state.read {
                    return OpenReadFileStreamEventOutput::Read(Vec::new()).try_into();
                }

                let n = u64::min(len, state.init_file_len - state.read);
                let mut buf = Vec::with_capacity(usize::min(n as usize, 2 * 1024 * 1024));

                state.file
                    .by_ref()
                    .take(n)
                    .read_to_end(&mut buf)?;

                state.read += buf.len() as u64;

                OpenReadFileStreamEventOutput::Read(buf).try_into()
            }).await?
        },
        OpenReadFileStreamEventInput::Close { id } => {
            tauri::async_runtime::spawn_blocking(move || {
                resources.close(id)?;
                OpenReadFileStreamEventOutput::Close(()).try_into()
            }).await?
        },
    }
}

struct FileResourceInner {
    file: std::fs::File,
    init_file_len: u64,
    read: u64,
}


#[derive(serde::Deserialize)]
#[serde(tag = "type")]
pub enum OpenReadFileStreamEventInput {
    Open {
        path: tauri_plugin_fs::SafeFilePath,

        #[serde(rename = "baseDir")]
        base_dir: Option<tauri::path::BaseDirectory>
    },
    Read {
        id: tauri::ResourceId,
        len: u64
    },
    Close {
        id: tauri::ResourceId,
    },
}

pub enum OpenReadFileStreamEventOutput {
    Open(tauri::ResourceId),
    Read(Vec<u8>),
    Close(()),
}

impl TryFrom<OpenReadFileStreamEventOutput> for tauri::ipc::Response {
    type Error = Error;

    fn try_from(v: OpenReadFileStreamEventOutput) -> Result<tauri::ipc::Response> {
        match v {
            OpenReadFileStreamEventOutput::Open(id) => {
                 let id_bytes = convert_rid_to_bytes(id);
                 Ok(tauri::ipc::Response::new(id_bytes))
            },
            OpenReadFileStreamEventOutput::Read(bytes) => {
                Ok(tauri::ipc::Response::new(bytes))
            },
            OpenReadFileStreamEventOutput::Close(()) => {
                Ok(tauri::ipc::Response::new(Vec::new()))
            }
        }
    }
}