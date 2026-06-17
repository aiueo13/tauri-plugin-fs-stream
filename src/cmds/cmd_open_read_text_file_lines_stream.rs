use super::*;
use crate::*;
use std::io::Read as _;


#[tauri::command]
pub async fn open_read_text_file_lines_stream<R: tauri::Runtime>(
    event: OpenReadTextFileLinesStreamEventInput,
    webview: tauri::Webview<R>,
    cmd_scope: tauri::ipc::CommandScope<Scope>,
    global_scope: tauri::ipc::GlobalScope<Scope>,
    resources: PluginFileResourcesState<'_, R>,
    config: PluginConfigState<'_>,
) -> Result<tauri::ipc::Response> {

    type FileReaderResource = std::sync::Mutex<FileReader>;


    let resources = std::sync::Arc::clone(&resources);
    let config = std::sync::Arc::clone(&config);

    match event {
        OpenReadTextFileLinesStreamEventInput::Open { path, base_dir, label, max_line_len, ignore_bom, freeze_size } => {
            tauri::async_runtime::spawn_blocking(move || {
                let path = resolve_path(
                    &webview,
                    &global_scope, 
                    &cmd_scope,
                    &config,
                    path,
                    base_dir
                )?;

                let file = std::fs::File::open(&path)?;
                let read_limit = match freeze_size {
                    true => Some(file.metadata()?.len()),
                    false => None,
                };
                let bom = match ignore_bom {
                    true => None,
                    false => bom_for_encoding_label(&label)
                };
                let line_breaks = line_breaks_for_encoding_label(&label);
                let max_line_len = std::num::NonZeroU64::new(max_line_len);

                let res = FileReader::new(file, max_line_len, line_breaks, bom, read_limit);
                let res: FileReaderResource = std::sync::Mutex::new(res);
                let id = resources.add(res)?;

                Ok(OpenReadFileStreamEventOutput::Open(id).try_into()?)
            }).await?
        }
        OpenReadTextFileLinesStreamEventInput::Read { id, len } => {
            tauri::async_runtime::spawn_blocking(move || -> Result<_> {
                let lines = resources
                    .get::<FileReaderResource>(id)?
                    .lock()?
                    .read_lines_framed(len)?;
                 
                Ok(OpenReadTextFileLinesStreamEventOutput::Read(lines).try_into()?)
            }).await?
        }
        OpenReadTextFileLinesStreamEventInput::Close { id } => {
            tauri::async_runtime::spawn_blocking(move || {
                resources.close(id)?;
                Ok(OpenReadTextFileLinesStreamEventOutput::Close(()).try_into()?)
            }).await?
        }
    }
}


#[derive(serde::Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all_fields = "camelCase")]
pub enum OpenReadTextFileLinesStreamEventInput {
    Open {
        path: tauri_plugin_fs::SafeFilePath,
        label: String,
        freeze_size: bool,
        base_dir: Option<tauri::path::BaseDirectory>,

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

pub enum OpenReadTextFileLinesStreamEventOutput {
    Open(tauri::ResourceId),
    Read(Vec<u8>),
    Close(()),
}

impl TryFrom<OpenReadTextFileLinesStreamEventOutput> for tauri::ipc::Response {
    type Error = Error;

    fn try_from(v: OpenReadTextFileLinesStreamEventOutput) -> Result<tauri::ipc::Response> {
        match v {
            OpenReadTextFileLinesStreamEventOutput::Open(id) => {
                 let id_bytes = convert_rid_to_bytes(id);
                 Ok(tauri::ipc::Response::new(id_bytes))
            },
            OpenReadTextFileLinesStreamEventOutput::Read(bytes) => {
                Ok(tauri::ipc::Response::new(bytes))
            },
            OpenReadTextFileLinesStreamEventOutput::Close(()) => {
                Ok(tauri::ipc::Response::new(Vec::new()))
            }
        }
    }
}


struct FileReader {
    file: std::io::BufReader<std::fs::File>,
    max_line_len: Option<std::num::NonZeroU64>,
    line_breaks: LineBreaks,
    bom: Option<&'static [u8]>,
    bom_handled: bool,
    read_limit: Option<u64>,
    read: u64
}

impl FileReader {

    fn new(
        file: std::fs::File,
        max_line_len: Option<std::num::NonZeroU64>,
        line_breaks: LineBreaks,
        bom: Option<&'static [u8]>,
        read_limit: Option<u64>,
    ) -> Self {

        Self {
            file: std::io::BufReader::new(file),
            max_line_len,
            line_breaks,
            bom,
            read_limit,
            bom_handled: false,
            read: 0
        }
    }

    /// この関数が返す bytes は以下の形式のレコードが連続したものであり、
    /// 各レコードが分断されることはない。
    /// 
    /// - err flag (u8, 0 = ok, 1 = err)
    /// - line break type (u8, 0 = null, 1 = '\n', 2 = '\r\n')
    /// - line bytes len (u64, big endian)
    /// - line bytes (variable bytes)
    /// 
    /// err flag が 0 の場合、正常にその行が読み込まれたことを指す。
    /// この場合、line bytes には BOM 処理されたテキストが格納される。
    /// 
    /// err flag が 1 の場合、その行でエラーが発生したことを示す。
    /// その場合、line bytes は utf-8 形式のエラーメッセージであり、
    /// この呼び出しでの最後の行となる。
    /// 
    /// エラー発生後の呼び出しの挙動は未定義。
    ///
    /// この関数は複数の行を先読みしてまとめて送信するため、
    /// 関数から直接エラーを返すのではなく、行単位でエラー情報を格納し、
    /// 対象行を明示的に読み込んだ際にエラーを発生させれるようにする。
    fn read_lines_framed(&mut self, threshold: u64) -> Result<Vec<u8>> {
        const ERR_FLAG_LEN: usize = 1;
        const LINE_BREAK_TYPE_LEN: usize = 1;
        const LINE_LEN_LEN: usize = 8;
        const HEADER_LEN: usize = ERR_FLAG_LEN + LINE_BREAK_TYPE_LEN + LINE_LEN_LEN;

        const FLAG_OK: u8 = 0;
        const FLAG_ERR: u8 = 1;
        const LINE_BREAK_NULL: u8 = 0;
        const LINE_BREAK_LF: u8 = 1;
        const LINE_BREAK_CRLF: u8 = 2;

        
        if self.read_limit.is_some_and(|l| l <= self.read) {
            return Ok(Vec::new())
        }

        let mut buf = Vec::with_capacity(usize::min(threshold as usize, 2 * 1024 * 1024));
        loop {
            let offset = buf.len();
            let header_offset = offset;
            let err_flag_offset = header_offset;
            let line_break_type_offset = err_flag_offset + ERR_FLAG_LEN;
            let line_len_offset = line_break_type_offset + LINE_BREAK_TYPE_LEN;
            let line_offset = line_len_offset + LINE_LEN_LEN;

            // header の場所を確保
            buf.extend_from_slice(&[0; HEADER_LEN]);

            let mut nlimit = u64::MAX;
            if let Some(read_limit) = self.read_limit {
                nlimit = u64::min(nlimit, read_limit.saturating_sub(self.read));
            } 
            if let Some(max_line_len) = self.max_line_len {
                // α は制限に含まない改行や BOM が取りうる最大の合計バイト数
                let mut alpha = self.line_breaks.lf.len() + self.line_breaks.cr.len();
                if !self.bom_handled {
                    alpha += self.bom.map(|b| b.len()).unwrap_or(0);
                }

                // 制限 + α のバイトを読み込み、
                // 制限を超えているかどうかで制限より大きいデータがあるかを判定する。
                let max_line_len = max_line_len.get().saturating_add(alpha as u64);

                nlimit = u64::min(nlimit, max_line_len);
            }

            // EOL ('\n', '\r\n') を検知するため '\n' が出るまで読み込む
            let nread = read_until_bytes(
                &mut self.file.by_ref().take(nlimit),
                &mut buf,
                &self.line_breaks.lf
            )?;
                    
            self.read += nread as u64;

            if nread == 0 || self.read_limit.is_some_and(|l| l <= self.read) {
                buf.truncate(offset);
                break
            }

            let mut line_len = nread;
            let mut line_break_type = LINE_BREAK_NULL;

            // 最後が EOL ('\n', '\r\n') で終わっていれば削除する。
            if self.line_breaks.lf.len() <= line_len && buf.ends_with(&self.line_breaks.lf) {
                buf.truncate(buf.len() - self.line_breaks.lf.len());
                line_len -= self.line_breaks.lf.len();
                line_break_type = LINE_BREAK_LF;
                if self.line_breaks.cr.len() <= line_len && buf.ends_with(&self.line_breaks.cr) {
                    buf.truncate(buf.len() - self.line_breaks.cr.len());
                    line_len -= self.line_breaks.cr.len();
                    line_break_type = LINE_BREAK_CRLF;
                }
            }
            // BOM をまだ処理していない場合、必要であれば削除する
            if !self.bom_handled {
                self.bom_handled = true;
                if let Some(bom) = self.bom {
                    if buf[line_offset..].starts_with(bom) {
                        buf.drain(line_offset..line_offset + bom.len());
                        line_len -= bom.len();
                    }
                }
            }

            // エラーとなるかの確認
            let checked = (|| -> Result<()> {
                if self.max_line_len.is_some_and(|i| i.get() < line_len as u64) {
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

        Ok(buf)
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
            lf: &[0x0A, 0x00],
            cr: &[0x0D, 0x00],
        },
        "utf-16be" => LineBreaks {
            lf: &[0x00, 0x0A],
            cr: &[0x00, 0x0D],
        },
        _ => LineBreaks {
            lf: &[b'\n'],
            cr: &[b'\r'],
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