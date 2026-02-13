use super::*;
use crate::*;
use std::io::Read as _;
use tauri::Manager as _;


#[tauri::command]
pub async fn open_read_text_file_lines_stream<R: tauri::Runtime>(
    event: EventInput,
    app: tauri::AppHandle<R>,
    cmd_scope: tauri::ipc::CommandScope<Scope>,
    global_scope: tauri::ipc::GlobalScope<Scope>,
) -> Result<tauri::ipc::Response> {

    type FileResource = PluginResource<std::sync::Mutex<FileResourceInner>>;

    struct FileResourceInner {
        file: std::io::BufReader<std::fs::File>,
        max_line_len: Option<std::num::NonZeroU64>,
        line_breaks: LineBreaks,
        bom: Option<&'static [u8]>,
        bom_handled: bool
    }


    match event {
        EventInput::Open { path, label, max_line_len, ignore_bom } => {
            validate_path_permission(&path, &app, &cmd_scope, &global_scope)?;

            tauri::async_runtime::spawn_blocking(move || {
                let file = std::fs::File::open(&path)?;
                let res = FileResourceInner {
                    file: std::io::BufReader::new(file),
                    max_line_len: std::num::NonZeroU64::new(max_line_len),
                    line_breaks: line_breaks_for_encoding_label(&label),
                    bom: match ignore_bom {
                        true => None,
                        false => bom_for_encoding_label(&label)
                    },
                    bom_handled: false
                };

                let id = app
                    .resources_table()
                    .add(FileResource::new(std::sync::Mutex::new(res)));

                Ok(OpenReadFileStreamEventOutput::Open(id).try_into()?)
            }).await?
        }
        EventInput::Read { id, len } => {
            tauri::async_runtime::spawn_blocking(move || -> Result<_> {
                let state = app.resources_table().get::<FileResource>(id)?.get();
                let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
                
                let line_break = state.line_breaks;
                let bom = state.bom;
                let max_line_len = state.max_line_len;
                let threshold = len;

                // この関数が返す bytes は以下の形式のレコードが連続したものであり、
                // 各レコードが分断されることはない。
                // 
                // - err flag (u8, 0 = ok, 1 = err)
                // - line break type (u8, 0 = null, 1 = '\n', 2 = '\r\n')
                // - line bytes len (u64, big endian)
                // - line bytes (variable bytes)
                // 
                // err flag が 0 の場合、正常にその行が読み込まれたことを指す。
                // この場合、line bytes には BOM 処理されたテキストが格納される。
                // 
                // err flag が 1 の場合、その行でエラーが発生したことを示す。
                // その場合、line bytes は utf-8 形式のエラーメッセージであり、
                // この呼び出しでの最後の行となる。
                // 
                // エラー発生後の呼び出しの挙動は未定義。
                //
                // この関数は複数の行を先読みしてまとめて送信するため、
                // 関数内で直接エラーを返すのではなく、行単位でエラー情報を伝え、
                // 対象行を明示的に読み込んだ際にエラーを発生させれるようにする。
                const ERR_FLAG_LEN: usize = 1;
                const LINE_BREAK_TYPE_LEN: usize = 1;
                const LINE_LEN_LEN: usize = 8;
                const HEADER_LEN: usize = ERR_FLAG_LEN + LINE_BREAK_TYPE_LEN + LINE_LEN_LEN;

                const FLAG_OK: u8 = 0;
                const FLAG_ERR: u8 = 1;
                const LINE_BREAK_NULL: u8 = 0;
                const LINE_BREAK_LF: u8 = 1;
                const LINE_BREAK_CRLF: u8 = 2;

                let mut buf = Vec::with_capacity(usize::min(len as usize, 2 * 1024 * 1024));
                loop {
                    let header_offset = buf.len();
                    let err_flag_offset = header_offset;
                    let line_break_type_offset = err_flag_offset + ERR_FLAG_LEN;
                    let line_len_offset = line_break_type_offset + LINE_BREAK_TYPE_LEN;
                    let line_offset = line_len_offset + LINE_LEN_LEN;

                    // header の場所を確保
                    buf.extend_from_slice(&[0; HEADER_LEN]);

                    // EOL ('\n', '\r\n') を検知するため '\n' が出るまで読み込む
                    let nread = match max_line_len {
                        None => read_until_bytes(
                            &mut state.file.by_ref(),
                            &mut buf,
                            &line_break.lf
                        )?,
                                
                        // 制限 + α のデータを読み込み、行が制限を超えているかを確認する。
                        // α があるのは制限丁度だと EOL　か制限で引っかかったのかわからないため。
                        Some(max) => read_until_bytes(
                            {
                                let mut alpha = line_break.lf.len() + line_break.cr.len();
                                if !state.bom_handled {
                                    alpha += bom.map(|b| b.len()).unwrap_or(0);
                                }

                                &mut state.file
                                    .by_ref()
                                    .take(max.get().saturating_add(alpha as u64))
                            },
                            &mut buf,
                            &line_break.lf
                        )?,
                    };

                    // EOF の場合
                    if nread == 0 {
                        // header 用に確保した分をキャンセル
                        buf.truncate(header_offset);
                        break
                    }

                    let mut line_len = nread;
                    let mut line_break_type = LINE_BREAK_NULL;

                    // 最後が EOL ('\n', '\r\n') で終わっていれば削除する。
                    if line_break.lf.len() <= line_len && buf.ends_with(&line_break.lf) {
                        buf.truncate(buf.len() - line_break.lf.len());
                        line_len -= line_break.lf.len();
                        line_break_type = LINE_BREAK_LF;
                        if line_break.cr.len() <= line_len && buf.ends_with(&line_break.cr) {
                            buf.truncate(buf.len() - line_break.cr.len());
                            line_len -= line_break.cr.len();
                            line_break_type = LINE_BREAK_CRLF;
                        }
                    }
                    // BOM をまだ処理していない場合、必要であれば削除する
                    if !state.bom_handled {
                        state.bom_handled = true;
                        if let Some(bom) = bom {
                            if buf[line_offset..].starts_with(bom) {
                                buf.drain(line_offset..line_offset + bom.len());
                                line_len -= bom.len();
                            }
                        }
                    }

                    // エラーとなるかの確認
                    let checked = (|| -> Result<()> {
                        if max_line_len.is_some_and(|i| i.get() < line_len as u64) {
                            return Err(Error::with("line length limit exceeded"));
                        }
                        Ok(())
                    })();
                        
                    if let Err(err) = checked {
                        let err_msg_bytes = err.to_string().into_bytes();

                        // header を設定
                        buf[err_flag_offset] = FLAG_ERR;
                        buf[line_break_type_offset] = LINE_BREAK_NULL;
                        buf[line_len_offset..(line_len_offset + LINE_LEN_LEN)]
                            .copy_from_slice(&u64::to_be_bytes(err_msg_bytes.len() as u64));

                        // データをエラーメッセージに差し替える
                        buf.truncate(line_offset);
                        buf.extend_from_slice(&err_msg_bytes);
                        break
                    }
                    else {
                        // header を設定
                        buf[err_flag_offset] = FLAG_OK;
                        buf[line_break_type_offset] = line_break_type;
                        buf[line_len_offset..(line_len_offset + LINE_LEN_LEN)]
                            .copy_from_slice(&u64::to_be_bytes(line_len as u64));

                        if threshold <= (buf.len() as u64) {
                            break
                        }
                    }
                }
                    
                Ok(EventOutput::Read(buf).try_into()?)
            }).await?
        }
        EventInput::Close { id } => {
            tauri::async_runtime::spawn_blocking(move || {
                let mut resources = app.resources_table();
                if resources.has(id) {
                    resources.close(id)?;
                }
                Ok(EventOutput::Close(()).try_into()?)
            }).await?
        }
    }
}


#[derive(serde::Deserialize)]
#[serde(tag = "type")]
pub enum EventInput {
    Open {
        path: String,
        label: String,

        #[serde(rename = "maxLineByteLength")]
        max_line_len: u64,

        #[serde(rename = "ignoreBOM")]
        ignore_bom: bool,
    },
    Read {
        id: tauri::ResourceId,
        len: u64,
    },
    Close {
        id: tauri::ResourceId,
    },
}

pub enum EventOutput {
    Open(tauri::ResourceId),
    Read(Vec<u8>),
    Close(()),
}

impl TryFrom<EventOutput> for tauri::ipc::Response {
    type Error = Error;

    fn try_from(v: EventOutput) -> Result<tauri::ipc::Response> {
        match v {
            EventOutput::Open(id) => {
                 let id_bytes = convert_rid_to_bytes(id);
                 Ok(tauri::ipc::Response::new(id_bytes))
            },
            EventOutput::Read(bytes) => {
                Ok(tauri::ipc::Response::new(bytes))
            },
            EventOutput::Close(()) => {
                Ok(tauri::ipc::Response::new(Vec::new()))
            }
        }
    }
}


/// label は `(new TextDecoder(encoding)).encoding` などで正規化された小文字のテキスト
fn bom_for_encoding_label(label: &str) -> Option<&'static [u8]> {
    // WEB 標準で定義されているエンコーディングのうち
    // UTF-8, UTF-16 LE/BE のみが BOM を持つ。
    match label {
        "utf-8" => Some(b"\xEF\xBB\xBF"),
        "utf-16le" => Some(b"\xFF\xFE"),
        "utf-16be" => Some(b"\xFE\xFF"),
        _ => None
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
struct LineBreaks {
    pub lf: &'static [u8],
    pub cr: &'static [u8],
}

/// label は `(new TextDecoder(encoding)).encoding` などで正規化された小文字のテキスト
fn line_breaks_for_encoding_label(label: &str) -> LineBreaks {
    // WEB 標準で定義されているエンコーディングのうち
    // UTF-16 LE/BE, ISO 2022-JP が ASCII 互換ではない。
    // ただし、ISO 2022-JP は ASCII と同じ改行コードである。
    match label {
        "utf-16le" => LineBreaks {
            lf:   &[0x0A, 0x00],
            cr:   &[0x0D, 0x00],
        },
        "utf-16be" => LineBreaks {
            lf:   &[0x00, 0x0A],
            cr:   &[0x00, 0x0D],
        },
        _ => LineBreaks {
            lf:   &[b'\n'],
            cr:   &[b'\r'],
        },
    }
}

fn read_until_bytes(
    r: &mut impl std::io::BufRead,
    buf: &mut Vec<u8>,
    bytes: &[u8]
) -> Result<usize> {

    let last_byte = *bytes.last().ok_or_else(|| Error::with("invalid empty bytes"))?;

    if bytes.len() == 1 {
        return Ok(r.read_until(last_byte, buf)?);
    }

    let mut total_n = 0;
    loop {
        let n = r.read_until(last_byte, buf)?;
        total_n += n;

        if n == 0 || buf.ends_with(bytes) {
            return Ok(total_n)
        }
    }
}