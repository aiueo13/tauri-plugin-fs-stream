use crate::*;
use super::*;
use std::io::Write as _;


#[tauri::command]
pub async fn open_write_file_stream<R: tauri::Runtime>(
    req: tauri::ipc::Request<'_>,
    webview: tauri::Webview<R>,
    cmd_scope: tauri::ipc::CommandScope<Scope>,
    global_scope: tauri::ipc::GlobalScope<Scope>,
    resources: PluginFileResourcesState<'_, R>,
    config: PluginConfigState<'_>,
) -> Result<OpenWriteFileStreamEventOutput> {

    type FileResource = std::sync::Mutex<std::fs::File>;

    
    let resources = std::sync::Arc::clone(&resources);
    let config = std::sync::Arc::clone(&config);
    let event: OpenWriteFileStreamEventInput = req.try_into()?;

    match event {
        OpenWriteFileStreamEventInput::Open { path, supports_raw_ipc_request_body, base_dir, open_options } => {
            tauri::async_runtime::spawn_blocking(move || {
                let path = resolve_path(
                    &webview,
                    &global_scope, 
                    &cmd_scope,
                    &config,
                    path,
                    base_dir,
                )?;

                let open_options = std::fs::OpenOptions::from(&open_options);
                let file = open_options.open(path)?;
                let res: FileResource = std::sync::Mutex::new(file);
                let id = resources.add(res)?;
                Ok(OpenWriteFileStreamEventOutput::Open { id, supports_raw_ipc_request_body })
            }).await?
        },
        OpenWriteFileStreamEventInput::Write { id, data } => {
            tauri::async_runtime::spawn_blocking(move || {
                resources
                    .get::<FileResource>(id)?
                    .lock()?
                    .write_all(&data)?;

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
        path: tauri_plugin_fs::SafeFilePath,
        base_dir: Option<tauri::path::BaseDirectory>,
        supports_raw_ipc_request_body: bool,
        open_options: OpenWriteFileStreamEventInputOpenOptions,
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
pub struct OpenWriteFileStreamEventInputOpenOptions {
    append: bool,
    create: bool,
    create_new: bool,

    #[allow(unused)]
    mode: Option<u32>,
}

impl From<&OpenWriteFileStreamEventInputOpenOptions> for std::fs::OpenOptions {

    fn from(value: &OpenWriteFileStreamEventInputOpenOptions) -> Self {
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
        macro_rules! get_args {
            () => {
                self.headers()
                    .get("tfps-cmd-args")
                    .ok_or_else(|| Error::missing_value("tfps-cmd-args"))
                    .map(|s| percent_encoding::percent_decode(s.as_ref()))
                    .and_then(|s| s.decode_utf8().map_err(Into::into))
                    .and_then(|s| serde_json::from_str(&s).map_err(Into::into))
            };
        }
        
        let cmd_type = self.headers()
            .get("tfps-cmd-type")
            .ok_or_else(|| Error::missing_value("tfps-cmd-type"))?
            .to_str()?;

        match cmd_type {
            "Open" => {
                // 呼び出し時に body として与えられた判定用の payload をチェックして
                // 生の body を受け取り可能かどうかを調べる。
                // <https://github.com/tauri-apps/tauri/issues/10573>
                let supports_raw_ipc_request_body = match self.body() {
                    tauri::ipc::InvokeBody::Json(_) => false,
                    tauri::ipc::InvokeBody::Raw(_) => true,
                };

                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                    path: tauri_plugin_fs::SafeFilePath,
                    base_dir: Option<tauri::path::BaseDirectory>,
                    open_options: OpenWriteFileStreamEventInputOpenOptions,
                }
                let args: Args = get_args!()?;

                Ok(OpenWriteFileStreamEventInput::Open {
                    path: args.path, 
                    supports_raw_ipc_request_body,
                    base_dir: args.base_dir,
                    open_options: args.open_options,
                })
            },
            "Write" => {
                let data = match self.body() {
                    tauri::ipc::InvokeBody::Raw(body) => {
                        body.clone()
                    },
                    tauri::ipc::InvokeBody::Json(body) => {
                        let data = body
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

                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                    id: tauri::ResourceId,
                }
                let args: Args = get_args!()?;
                let id = args.id;

                Ok(OpenWriteFileStreamEventInput::Write { id, data })
            },
            "Close" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                    id: tauri::ResourceId,
                }
                let args: Args = get_args!()?;
                let id = args.id;

                Ok(OpenWriteFileStreamEventInput::Close { id })
            },
            value => Err(Error::invalid_value("eventType", value))
        }
    }
}

#[derive(serde::Serialize)]
#[serde(untagged)]
#[serde(rename_all_fields = "camelCase")]
pub enum OpenWriteFileStreamEventOutput {
    Open {
        id: tauri::ResourceId,
        supports_raw_ipc_request_body: bool
    },
    Write(()),
    Close(()),
}