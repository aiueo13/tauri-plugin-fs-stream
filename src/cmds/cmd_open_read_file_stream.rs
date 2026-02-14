use crate::*;
use super::*;
use tauri::Manager as _;
use std::io::Read as _;


#[tauri::command]
pub async fn open_read_file_stream<R: tauri::Runtime>(
    event: OpenReadFileStreamEventInput,
    app: tauri::AppHandle<R>,
    cmd_scope: tauri::ipc::CommandScope<Scope>,
    global_scope: tauri::ipc::GlobalScope<Scope>,
) -> Result<tauri::ipc::Response> {

    type FileResource = PluginResource<std::sync::Mutex<std::fs::File>>;
    
    
    match event {
        OpenReadFileStreamEventInput::Open { path } => {
            validate_path_permission(&path, &app, &cmd_scope, &global_scope)?;
            
            tauri::async_runtime::spawn_blocking(move || {
                let file = std::fs::File::open(&path)?;
                let id = app
                    .resources_table()
                    .add(FileResource::new(std::sync::Mutex::new(file)));

                Ok(OpenReadFileStreamEventOutput::Open(id).try_into()?)
            }).await?
        },
        OpenReadFileStreamEventInput::Read { id, len } => {
            tauri::async_runtime::spawn_blocking(move || -> Result<_> {
                let file = app.resources_table().get::<FileResource>(id)?.get();
                let mut file = file.lock()?;
                
                let init_cap = usize::min(len as usize, 2 * 1024 * 1024);
                let mut buf = Vec::with_capacity(init_cap);
                
                file.by_ref()
                    .take(len)
                    .read_to_end(&mut buf)?;

                Ok(OpenReadFileStreamEventOutput::Read(buf).try_into()?)
            }).await?
        },
        OpenReadFileStreamEventInput::Close { id } => {
            tauri::async_runtime::spawn_blocking(move || {
                let mut resources = app.resources_table();
                if resources.has(id) {
                    resources.close(id)?;
                }
                Ok(OpenReadFileStreamEventOutput::Close(()).try_into()?)
            }).await?
        },
    }
}


#[derive(serde::Deserialize)]
#[serde(tag = "type")]
pub enum OpenReadFileStreamEventInput {
    Open {
        path: String,
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