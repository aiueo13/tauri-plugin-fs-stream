use crate::*;
use super::*;
use std::io::Write as _;


#[tauri::command]
pub async fn open_write_file_stream<R: tauri::Runtime>(
    req: tauri::ipc::Request<'_>,
    app: tauri::AppHandle<R>,
    cmd_scope: tauri::ipc::CommandScope<Scope>,
    global_scope: tauri::ipc::GlobalScope<Scope>,
    resources: PluginResourcesState<'_, R>,
) -> Result<OpenWriteFileStreamEventOutput> {

    type FileResource = PluginResource<std::sync::Mutex<std::fs::File>>;

    
    let resources = std::sync::Arc::clone(&resources);
    let event: OpenWriteFileStreamEventInput = req.try_into()?;

    match event {
        OpenWriteFileStreamEventInput::Open { path, options } => {
            let path = resolve_path(options.base_dir, path, &app)?;
            validate_path_permission(&path, &app, &cmd_scope, &global_scope)?;

            tauri::async_runtime::spawn_blocking(move || {
                let file_options: std::fs::OpenOptions = (&options).into();
                let file = file_options.open(path)?;
                let id = resources.add(FileResource::new(std::sync::Mutex::new(file)))?;
                Ok(OpenWriteFileStreamEventOutput::Open(id))
            }).await?
        },
        OpenWriteFileStreamEventInput::Write { id, data } => {
            tauri::async_runtime::spawn_blocking(move || {
                let file = resources.get::<FileResource>(id)?.get();
                let mut file = file.lock()?;
                file.write_all(&data)?;
                Ok(OpenWriteFileStreamEventOutput::Write(()))
            }).await?
        },
        OpenWriteFileStreamEventInput::Close { id } => {
            tauri::async_runtime::spawn_blocking(move || {   
                resources.close(id)?;
                Ok(OpenWriteFileStreamEventOutput::Close(()))
            }).await?
        },
    }
}


pub enum OpenWriteFileStreamEventInput {
    Open {
        path: String,
        options: OpenWriteFileStreamEventInputFileOptions
    },
    Write {
        id: tauri::ResourceId,
        data: Vec<u8>,
    },
    Close {
        id: tauri::ResourceId,
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenWriteFileStreamEventInputFileOptions {
    append: bool,
    create: bool,
    create_new: bool,
    #[allow(unused)]
    mode: Option<u32>,
    base_dir: Option<tauri::path::BaseDirectory>
}

impl From<&OpenWriteFileStreamEventInputFileOptions> for std::fs::OpenOptions {

    fn from(value: &OpenWriteFileStreamEventInputFileOptions) -> Self {
        let mut options = std::fs::OpenOptions::new();

        #[cfg(unix)] {
            use std::os::unix::fs::OpenOptionsExt;
            if let Some(mode) = &value.mode {
                options.mode(*mode);
            }
        }

        if value.append {
            options.append(true);
        }
        else {  
            options.truncate(true);
        }
        options.create(value.create);
        options.create_new(value.create_new);
        options.write(true);
        options
    }
}

impl<'a> TryInto<OpenWriteFileStreamEventInput> for tauri::ipc::Request<'a> {
    type Error = Error;

    fn try_into(self) -> std::result::Result<OpenWriteFileStreamEventInput, Self::Error> {
        let get_header_value = |header_name: &str| -> Result<_> {
            self.headers()
                .get(header_name)
                .ok_or_else(|| Error::missing_value(header_name))
        };
        
        let event_type = get_header_value("eventType")?.to_str()?;

        match event_type {
            "Open" => {
                let tauri::ipc::InvokeBody::Json(body) = self.body() else {
                    return Err(Error::with("invalid body"))
                };

                let path = body
                    .get("path")
                    .ok_or_else(|| Error::missing_value("path"))?
                    .as_str()
                    .ok_or_else(|| Error::invalid_type("path"))?
                    .to_string();

                let options = body
                    .get("options")
                    .ok_or_else(|| Error::missing_value("options"))?
                    .as_str()
                    .ok_or_else(|| Error::invalid_type("options"))?;
                
                let options = serde_json::from_str(options)?;

                Ok(OpenWriteFileStreamEventInput::Open { path, options })
            },
            "Write" => {
                let id = get_header_value("id")?
                    .to_str()?
                    .parse::<u32>()?;

                let data = match self.body() {
                    tauri::ipc::InvokeBody::Raw(bytes) => {
                        bytes.clone()
                    },
                    tauri::ipc::InvokeBody::Json(json) => {
                        let data = json
                            .get("data")
                            .ok_or_else(|| Error::missing_value("data"))?
                            .as_str()
                            .ok_or_else(|| Error::invalid_type("data"))?;

                        let b64 = match data.starts_with("data:") {
                            // data URL
                            true => {
                                let comma_i = data
                                    .find(",")
                                    .ok_or_else(|| Error::with("invalid Data URL"))?;

                                let (_, b64) = data.split_at(comma_i + 1);
                                b64
                            },
                            // base64
                            false => data,
                        };

                        // TODO: データが大きい場合は別スレッドに逃す
                        use base64::engine::Engine;
                        base64::engine::general_purpose::STANDARD.decode(b64)?
                    },
                };

                Ok(OpenWriteFileStreamEventInput::Write { id, data })
            },
            "Close" => {
                let id = get_header_value("id")?
                    .to_str()?
                    .parse::<u32>()?;

                Ok(OpenWriteFileStreamEventInput::Close { id })
            },
            value => Err(Error::invalid_value("eventType", value))
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(untagged)]
pub enum OpenWriteFileStreamEventOutput {
    Open(tauri::ResourceId),
    Write(()),
    Close(()),
}
