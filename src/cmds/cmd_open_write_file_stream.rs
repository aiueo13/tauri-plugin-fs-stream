use crate::*;
use super::*;
use tauri::Manager as _;
use std::io::Write as _;


#[tauri::command]
pub async fn open_write_file_stream<R: tauri::Runtime>(
    req: tauri::ipc::Request<'_>,
    app: tauri::AppHandle<R>,
    cmd_scope: tauri::ipc::CommandScope<Scope>,
    global_scope: tauri::ipc::GlobalScope<Scope>,
) -> Result<OpenWriteFileStreamEventOutput> {

    type FileResource = PluginResource<std::sync::Mutex<std::fs::File>>;

    
    let event: OpenWriteFileStreamEventInput = req.try_into()?;

    match event {
        OpenWriteFileStreamEventInput::Open { path } => {
            validate_path_permission(&path, &app, &cmd_scope, &global_scope)?;

            tauri::async_runtime::spawn_blocking(move || {
                let file = std::fs::File::create(&path)?;
                let id = app
                    .resources_table()
                    .add(FileResource::new(std::sync::Mutex::new(file)));

                Ok(OpenWriteFileStreamEventOutput::Open(id))
            }).await?
        },
        OpenWriteFileStreamEventInput::Write { id, data } => {
            tauri::async_runtime::spawn_blocking(move || {
                let file = app.resources_table().get::<FileResource>(id)?.get();
                let mut file = file.lock()?;
                file.write_all(&data)?;
                Ok(OpenWriteFileStreamEventOutput::Write(()))
            }).await?
        },
        OpenWriteFileStreamEventInput::Close { id } => {
            tauri::async_runtime::spawn_blocking(move || {
                let mut resources = app.resources_table();
                if resources.has(id) {
                    resources.close(id)?;
                }
                Ok(OpenWriteFileStreamEventOutput::Close(()))
            }).await?
        },
    }
}


pub enum OpenWriteFileStreamEventInput {
    Open {
        path: String,
    },
    Write {
        id: tauri::ResourceId,
        data: Vec<u8>,
    },
    Close {
        id: tauri::ResourceId,
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
                let path = get_header_value("path")?.to_str()?;
                let path = percent_encoding::percent_decode_str(path)
                    .decode_utf8()?
                    .to_string();

                Ok(OpenWriteFileStreamEventInput::Open { path })
            },
            "Write" => {
                let id = get_header_value("id")?.to_str()?.parse::<u32>()?;
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
                let id = get_header_value("id")?.to_str()?.parse::<u32>()?;

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
