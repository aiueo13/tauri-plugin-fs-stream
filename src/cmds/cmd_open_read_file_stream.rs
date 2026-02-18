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

    type FileReaderResource = PluginResource<std::sync::Mutex<FileReader>>;

    
    let resources = std::sync::Arc::clone(&resources);
    let config = std::sync::Arc::clone(&config);
    
    match event {
        OpenReadFileStreamEventInput::Open { path, base_dir, freeze_size } => {
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
                let read_limit = match freeze_size {
                    true => Some(file.metadata()?.len()),
                    false => None,
                };

                let res = FileReader::new(file, read_limit);
                let res = FileReaderResource::new(std::sync::Mutex::new(res));
                let id = resources.add(res)?;

                OpenReadFileStreamEventOutput::Open(id).try_into()
            }).await?
        },
        OpenReadFileStreamEventInput::Read { id, len } => {
            tauri::async_runtime::spawn_blocking(move || -> Result<_> {
                let data = resources
                    .get::<FileReaderResource>(id)?
                    .get()
                    .lock()?
                    .read_chunk(len)?;
                
                OpenReadFileStreamEventOutput::Read(data).try_into()
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


#[derive(serde::Deserialize)]
#[serde(tag = "type")]
pub enum OpenReadFileStreamEventInput {
    Open {
        path: tauri_plugin_fs::SafeFilePath,

        #[serde(rename = "freezeSize")]
        freeze_size: bool,

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


struct FileReader {
    file: std::fs::File,
    read_limit: Option<u64>,
    read: u64,
}

impl FileReader {

    fn new(file: std::fs::File, read_limit: Option<u64>) -> Self {
        FileReader {
            read_limit,
            read: 0,
            file,
        }
    }

    fn read_chunk(&mut self, len: u64) -> Result<Vec<u8>> {
        if self.read_limit.is_some_and(|l| l <= self.read) {
            return Ok(Vec::new())
        }

        let mut nlimit = len;
        if let Some(read_limit) = self.read_limit {
            nlimit = u64::min(nlimit, read_limit.saturating_sub(self.read));
        }

        let mut buf = Vec::with_capacity(usize::min(nlimit as usize, 2 * 1024 * 1024));

        let nread = self.file
            .by_ref()
            .take(nlimit)
            .read_to_end(&mut buf)?;

        self.read += nread as u64;

        Ok(buf)
    }
}
