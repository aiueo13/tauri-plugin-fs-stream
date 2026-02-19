use crate::*;
use super::*;
use std::io::Write as _;
use std::str::FromStr as _;


#[tauri::command]
pub async fn open_write_file_stream<R: tauri::Runtime>(
    req: tauri::ipc::Request<'_>,
    webview: tauri::Webview<R>,
    cmd_scope: tauri::ipc::CommandScope<Scope>,
    global_scope: tauri::ipc::GlobalScope<Scope>,
    resources: PluginResourcesState<'_, R>,
    config: PluginConfigState<'_>,
) -> Result<OpenWriteFileStreamEventOutput> {

    type FileResource = std::sync::Mutex<std::fs::File>;

    
    let resources = std::sync::Arc::clone(&resources);
    let config = std::sync::Arc::clone(&config);
    let event: OpenWriteFileStreamEventInput = req.try_into()?;

    match event {
        OpenWriteFileStreamEventInput::Open { path, options, supports_raw_ipc_request_body } => {
            let path = resolve_path(
                &webview,
                &global_scope, 
                &cmd_scope,
                &config,
                path,
                options.base_dir
            )?;

            tauri::async_runtime::spawn_blocking(move || {
                let file_options = std::fs::OpenOptions::from(&options);
                let file = file_options.open(path)?;
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
        supports_raw_ipc_request_body: bool,
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
                // 呼び出し時に body として与えられた判定用の payload をチェックして
                // 生の body を受け取り可能かどうかを調べる。
                // <https://github.com/tauri-apps/tauri/issues/10573>
                let supports_raw_ipc_request_body = match self.body() {
                    tauri::ipc::InvokeBody::Json(_) => false,
                    tauri::ipc::InvokeBody::Raw(_) => true,
                };

                let path = get_header_value("path")
                    .map(|p| percent_encoding::percent_decode(p.as_ref()))
                    .and_then(|p| p.decode_utf8().map_err(Into::into))
                    .and_then(|p| tauri_plugin_fs::SafeFilePath::from_str(&p).map_err(Into::into))?;
               
                let options = get_header_value("options")
                    .map(|s| percent_encoding::percent_decode(s.as_ref()))
                    .and_then(|s| s.decode_utf8().map_err(Into::into))
                    .and_then(|s| serde_json::from_str(&s).map_err(Into::into))?;

                Ok(OpenWriteFileStreamEventInput::Open { path, options, supports_raw_ipc_request_body })
            },
            "Write" => {
                let id = get_header_value("id")?
                    .to_str()?
                    .parse::<u32>()?;

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
    Open {
        id: tauri::ResourceId,

        #[serde(rename="supportsRawIpcRequestBody")]
        supports_raw_ipc_request_body: bool
    },
    Write(()),
    Close(()),
}

