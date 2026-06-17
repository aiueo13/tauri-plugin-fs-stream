import { invoke } from '@tauri-apps/api/core'
import { BaseDirectory } from '@tauri-apps/api/path'
import { createReadableStream, createWritableStream } from 'create-web-stream'


export type OpenReadFileStreamOptions = {

	/**
	 * Buffer size, in bytes, used when sending data from the backend to the frontend.
	 * 
	 * @remarks
	 * IPC calls are relatively expensive, so larger buffer sizes are generally more efficient. 
	 * However, setting this value too high may cause the UI to freeze or result in out-of-memory errors.
	 * 
	 * @defaultValue `524288` (512 KiB)
	 */
	bufferByteLength?: number,

	/**
	 * Indicates whether to limit the read stream to the file size at the moment of opening.
	 *
	 * @remarks
	 * This serves as a safety mechanism to prevent infinite loops if the stream is piped back into the same file (read/write cycle).
	 * 
	 * The behavior is as follows:
	 * - `true`: The stream acts as a fixed snapshot, stopping exactly when the initial file size is reached. 
	 * - `false`: The stream reads until the underlying file returns EOF. This allows reading data appended to the file while the stream is open.
	 *
	 * @default `true`
	 */
	freezeSize?: boolean,

	/**
	 * `AbortSignal` that allows the read operation to be aborted.
	 * 
	 * @remarks
	 * When aborted, the stream enters an errored state, all subsequent read operations fail,
	 * and the underlying file resources are released immediately.
	 */
	signal?: AbortSignal,

	/**
	 * Base directory to resolve the relative `path` against.
	 */
	baseDir?: BaseDirectory
}

/**
 * Opens a file in read-only mode and resolves to a {@link https://developer.mozilla.org/ja/docs/Web/API/ReadableStream | ReadableStream}.
 * 
 * @remarks
 * The caller is responsible for releasing the returned stream.
 * The stream is released in the following cases:
 * - When the stream or its reader is canceled. 
 * - When all data has been successfully read from the stream.
 * - When a read operation fails with an error.
 * - When the provided `AbortSignal` is aborted.
 * - When `closeAllFileStreams` is called.
 * 
 * @param path - File path or file scheme URL to read. 
 * @param options - Optional settings: `bufferByteLength`, `signal`, `freezeSize`, `baseDir`. See `OpenReadFileStreamOptions` for details.
 * 
 * @returns Promise that resolves to a `ReadableStream<Uint8Array<ArrayBuffer>>` backed by the file opened in read-only mode. This stream maintains a one-to-one correspondence with the OS handle (file descriptor on Unix or file handle on Windows).
 * 
 * @throws
 * The returned Promise rejects with an error in the following cases:
 * - When the entry is a directory, not a file.
 * - When the file does not exist.
 * - When the app does not have read permissions for the file.
 * - When an unexpected error occurred.
 */
export async function openReadFileStream(
	path: string | URL,
	options?: OpenReadFileStreamOptions
): Promise<ReadableStream<Uint8Array<ArrayBuffer>>> {

	throwIfAborted(options?.signal)
	const bufferByteLength = mapBufferByteLengthForInput(options?.bufferByteLength)
	const baseDir = options?.baseDir
	const freezeSize = options?.freezeSize ?? true
	const { open, read, close } = resolveCmdReadFileStream("plugin:fs-stream|open_read_file_stream")

	try {
		await open({
			path: mapFsPathForInput(path),
			baseDir,
			freezeSize,
		})

		return createReadableStream(
			{
				read: () => read(bufferByteLength),
				release: () => close()
			},
			{ signal: options?.signal }
		)
	}
	catch (e) {
		await close().catch(() => { })
		throw e
	}
}


export type OpenReadTextFileLinesStreamOptions = {

	/**
	 * Text encoding used to decode the data, such as `"utf-8"`, `"shift_jis"`, or `"iso-8859-2"`.
	 * 
	 * @see {@link https://developer.mozilla.org/ja/docs/Web/API/Encoding_API/Encodings | available encodings}
	 * @see {@link https://developer.mozilla.org/en-US/docs/Web/API/TextDecoder/TextDecoder#label | TextDecoder's label option}
	 * 
	 * @defaultValue `"utf-8"`.
	 */
	encoding?: string,

	/**
	 * Indicates whether decoding errors are treated as fatal.
	 *
	 * @remarks
	 * The behavior is as follows:
	 * - `false`: Invalid byte sequences are replaced with U+FFFD (`�`) and decoding continues.
	 * - `true`: An error is thrown when an invalid byte sequence is encountered.
	 * 
	 * @see {@link https://developer.mozilla.org/en-US/docs/Web/API/TextDecoder/TextDecoder#fatal | WebAPI TextDecoder's fatal option}
	 *
	 * @defaultValue `false`
	 */
	fatal?: boolean,

	/**
	 * Indicates whether to ignore a leading BOM (Byte Order Mark).
	 *
	 * @remarks
	 * The behavior is as follows:
	 * - `false`: The leading BOM is automatically stripped from the decoded result.
	 * - `true`: The leading BOM is preserved as a regular character.
	 * 
	 * @see {@link https://developer.mozilla.org/en-US/docs/Web/API/TextDecoder/TextDecoder#ignorebom | WebAPI TextDecoder's ignoreBOM option}
	 *
	 * @defaultValue `false`
	 */
	ignoreBOM?: boolean,

	/**
	 * Buffer size, in bytes, used when sending data from the backend to the frontend.
	 * 
	 * @remarks
	 * IPC calls are relatively expensive, so larger buffer sizes are generally more efficient. 
	 * However, setting this value too high may cause the UI to freeze or result in out-of-memory errors.
	 * 
	 * This value is not guaranteed to be strictly respected.
	 * If a single line exceeds this size, more bytes may be sent in a single IPC transmission.
	 * To prevent OOM errors, use `maxLineByteLength`.
	 * 
	 * @defaultValue `524288` (512 KiB)
	 */
	bufferByteLength?: number,

	/**
	 * Maximum byte length of a line before decoding.
	 * 
	 * @remarks
	 * If a line exceeds this limit, an error is thrown. 
	 * This prevents OOM errors when reading minified files or binaries.
	 * 
	 * This excluding line break characters and the initial BOM (if present). 
	 * 
	 * @defaultValue `0` (unlimited)
	 */
	maxLineByteLength?: number,

	/**
	 * Indicates whether to limit the read stream to the file size at the moment of opening.
	 *
	 * @remarks
	 * This serves as a safety mechanism to prevent infinite loops if the stream is piped back into the same file (read/write cycle).
	 * 
	 * The behavior is as follows:
	 * - `true`: The stream acts as a fixed snapshot, stopping exactly when the initial file size is reached. 
	 * - `false`: The stream reads until the underlying file returns EOF. This allows reading data appended to the file while the stream is open.
	 *
	 * @default `true`
	 */
	freezeSize?: boolean,

	/**
	 * `AbortSignal` that allows the read operation to be aborted.
	 * 
	 * @remarks
	 * When aborted, the stream enters an errored state, all subsequent read operations fail,
	 * and the underlying file resources are released immediately.
	 */
	signal?: AbortSignal,

	/**
	 * Base directory to resolve the relative `path` against.
	 */
	baseDir?: BaseDirectory,
}

export type OpenReadTextFileLinesStreamItem = {

	/**
	 * Text of the current line.
	 * 
	 * @remarks
	 * This value excludes line break characters.
	 * If needed, use `lineBreak`.
	 */
	line: string,

	/**
	 * Line break characters at the end of the current line.
	 * 
	 * @remarks
	 * One of: `"\n"`, `"\r\n"`, `null`.
	 * 
	 * This value is `null` 
	 * if the current line is the last line and does not end with a line break.
	 */
	lineBreak: "\n" | "\r\n" | null
}

/**
 * Opens a text file in read-only mode and resolves to a {@link https://developer.mozilla.org/ja/docs/Web/API/ReadableStream | ReadableStream} of text lines.
 * 
 * @remarks
 * The returned stream yields decoded text line by line.
 * For the structure of each item, see `OpenReadTextFileLinesStreamItem`.
 * 
 * The caller is responsible for releasing the returned stream.
 * The stream is released in the following cases:
 * - When the stream or its reader is canceled. 
 * - When all data has been successfully read from the stream.
 * - When a read operation fails with an error. 
 * - When the provided `AbortSignal` is aborted.
 * - When `closeAllFileStreams` is called.
 *
 * @param uri - File path or file scheme URL to read. 
 * @param options - Optional settings: `encoding`, `fatal`, `ignoreBOM`, `maxLineByteLength`, `bufferByteLength`, `signal`, `freezeSize`, `baseDir`. See `OpenReadTextFileLinesStreamOptions` for details.
 * 
 * @returns Promise that resolves to a `ReadableStream<OpenReadTextFileLinesStreamItem>` backed by the file opened in read-only mode. This stream maintains a one-to-one correspondence with the OS handle (file descriptor on Unix or file handle on Windows).
 *
 * @throws 
 * The returned Promise rejects with an error in the following cases:
 * - When the entry is a directory, not a file.
 * - When the file does not exist.
 * - When the app does not have read permissions for the file.
 * - When an unexpected error occurred.
 */
export async function openReadTextFileLinesStream(
	path: string | URL,
	options?: OpenReadTextFileLinesStreamOptions,
): Promise<ReadableStream<OpenReadTextFileLinesStreamItem>> {

	throwIfAborted(options?.signal)
	const maxLineByteLength = mapMaxLineByteLength(options?.maxLineByteLength)
	const bufferSize = mapBufferByteLengthForInput(options?.bufferByteLength)
	const label = mapEncodingLabelForInput(options?.encoding)
	const baseDir = options?.baseDir
	const freezeSize = options?.freezeSize ?? true
	const fatal = options?.fatal ?? false
	const ignoreBOM = options?.ignoreBOM ?? false
	const { open, read, close } = resolveCmdReadFileStream("plugin:fs-stream|open_read_text_file_lines_stream")

	try {
		await open({
			path: mapFsPathForInput(path),
			baseDir,
			freezeSize,
			maxLineByteLength,
			ignoreBOM,
			label,
		})

		return createTextLinesReadableStream(
			{
				read: () => read(bufferSize),
				release: close
			},
			{ label, fatal },
			options?.signal
		)
	}
	catch (e) {
		await close().catch(() => { })
		throw e
	}
}


export type OpenWriteFileStreamOptions = {

	/**
	 * Buffer size, in bytes, used when sending data from the frontend to the backend.
	 * 
	 * @remarks
	 * IPC calls are relatively expensive, so larger buffer sizes are generally more efficient. 
	 * However, setting this value too high may cause the UI to freeze or result in out-of-memory errors.
	 * 
	 * @defaultValue `524288` (512 KiB)
	 */
	bufferByteLength?: number,

	/**
	 * `AbortSignal` that allows the write operation to be aborted.
	 * 
	 * @remarks
	 * When aborted, the stream enters an errored state, all subsequent write operations fail,
	 * and the underlying file resources are released immediately.
	 */
	signal?: AbortSignal,

	/**
	 * Indicates whether to append data to the end of the file.
	 * 
	 * @remarks
	 * The behavior is as follows:
	 * - `true`: Preserves the existing data and writes the new data to the end of the file.
	 * - `false`: Truncates the existing data and writes the new data.
	 * 
	 * @defaultValue `false`
	 */
	append?: boolean,

	/**
	 * Indicates whether to create a new file if it does not exist.
	 *
	 * @defaultValue `true`
	 */
	create?: boolean,

	/**
	 * Indicates whether a new file must be created. 
	 * 
	 * @remarks
	 * If set to `true`, the operation will fail with an error if the file already exists.
	 * 
	 * @defaultValue `false`
	 */
	createNew?: boolean,

	/**
	 * File mode bits (permissions) applied when creating a new file.
	 * 
	 * @remarks
	 * This option is ignored on Windows.
	 * 
	 * @see {@link https://doc.rust-lang.org/std/os/unix/fs/trait.OpenOptionsExt.html#tymethod.mode | std::os::unix::fs::OpenOptionsExt::mode}
	 */
	mode?: number,

	/**
	 * Base directory to resolve the relative `path` against.
	 */
	baseDir?: BaseDirectory
}

/**
 * Opens a file in write-only mode and resolves to a {@link https://developer.mozilla.org/ja/docs/Web/API/WritableStream | WritableStream}.  
 * 
 * @remarks
 * The caller is responsible for releasing the returned stream.
 * The stream is released in the following cases:
 * - When the stream or its writer is closed.
 * - When the stream or its writer is aborted.
 * - When a write operation fails with an error.
 * - When the provided `AbortSignal` is aborted.
 * - When `closeAllFileStreams` is called.
 * 
 * @param uri - File path or file scheme URL write to. 
 * @param options - Optional settings: `bufferByteLength`, `signal`, `append`, `create`, `createNew`, `mode`, `baseDir`. See `OpenWriteFileStreamOptions` for details.
 * 
 * @returns Promise that resolves to a `WritableStream<Uint8Array<ArrayBufferLike>>` backed by the file opened in write-only mode. This stream maintains a one-to-one correspondence with the OS handle (file descriptor on Unix or file handle on Windows).
 *
 * @throws
 * The returned Promise rejects with an error in the following cases:
 * - When the entry is a directory, not a file.
 * - When the app does not have write permissions for the file.
 * - When `options.create` is `false`, and the file does not exist.
 * - When `options.createNew` is `true`, and the file already exists.
 * - When an unexpected error occurred.
 */
export async function openWriteFileStream(
	path: string | URL,
	options?: OpenWriteFileStreamOptions
): Promise<WritableStream<Uint8Array<ArrayBufferLike>>> {

	throwIfAborted(options?.signal)
	const bufferByteLength = mapBufferByteLengthForInput(options?.bufferByteLength)
	const { open, write, close } = resolveCmdWriteFileStream("plugin:fs-stream|open_write_file_stream")

	try {
		await open({
			path: mapFsPathForInput(path),
			baseDir: options?.baseDir,
			openOptions: {
				append: options?.append ?? false,
				create: options?.create ?? true,
				createNew: options?.createNew ?? false,
				mode: options?.mode,
			}
		})

		return createWritableStream(
			{
				write,
				release: close
			},
			{
				signal: options?.signal,
				bufferSize: bufferByteLength,
				useBufferView: true,
				strictBufferSize: false,
			}
		)
	}
	catch (e) {
		await close().catch(() => { })
		throw e
	}
}


/**
 * Forcibly disposes of all file streams.
 *
 * @remarks
 * All backend file resources owned by stream instances
 * created by this plugin are disconnected from the frontend and released.
 * 
 * After this operation,
 * any read or write attempts on existing streams will result in an error, 
 * except for buffering in the frontend.
 * 
 * This affects streams created by the following methods:
 * - `openReadFileStream`
 * - `openReadTextFileLinesStream`
 * - `openWriteFileStream`
 * 
 * @returns Promise that resolves when the operation completes successfully.
 */
export async function closeAllFileStreams(): Promise<void> {
	await invoke("plugin:fs-stream|close_all_file_streams")
}

/**
 * Retrieves the number of currently active file streams.
 * 
 * @remarks
 * This counts all backend file resources owned by stream instances created by this plugin
 * that have not yet been disconnected from the frontend and released.
 * 
 * This applies to streams created by the following methods:
 * - `openReadFileStream`
 * - `openReadTextFileLinesStream`
 * - `openWriteFileStream`
 * 
 * @returns Promise that resolves to a number of currently active file streams.
 */
export async function countAllFileStreams(): Promise<number> {
	return await invoke("plugin:fs-stream|count_all_file_streams")
}


/** 512 KiB */
const DEFAULT_BUFFER_SIZE_FOR_IPC = 512 * 1024;

function mapBufferByteLengthForInput(s?: number): number {
	const bufferSize = s ?? DEFAULT_BUFFER_SIZE_FOR_IPC
	if (!isNonzeroSafeInt(bufferSize)) {
		throw new Error("Invalid bufferByteLength: expected a non-zero safe unsigned integer (1..Number.MAX_SAFE_INTEGER)")
	}
	return bufferSize
}

function mapEncodingLabelForInput(label?: string): string {
	try {
		return (new TextDecoder(label)).encoding
	}
	catch {
		throw new RangeError("Bad encoding label")
	}
}

function mapMaxLineByteLength(s?: number): number {
	if (s == null) return 0

	if (!Number.isSafeInteger(s) || s < 0) {
		throw new Error("Invalid maxLineByteLength: expected a safe unsigned integer");
	}

	return s
}

function mapFsPathForInput(path: string | URL): string {
	return path instanceof URL ? path.toString() : path
}

type CmdReadFileStreamHandler = {
	open: (args: Record<string, unknown>) => Promise<void>
	read: (len: number) => Promise<Uint8Array<ArrayBuffer> | null>,
	close: () => Promise<void>,
}
function resolveCmdReadFileStream(cmdName: string): CmdReadFileStreamHandler {
	// Tauri IPC の制約により、戻り値は全て ArrayBuffer 型となる。
	type CmdEvents = {
		Open: Record<string, unknown>,
		Read: { id: number, len: number },
		Close: { id: number },
	}
	type CmdType = keyof CmdEvents
	type CmdInput<T extends CmdType> = CmdEvents[T]
	function cmd<T extends CmdType>(type: T, input: CmdInput<T>): Promise<ArrayBuffer> {
		return invoke(cmdName, { event: { type, ...input } })
	}


	let id: Promise<number> | null = null

	return {
		open: async (args) => {
			if (id !== null) throw new Error("File already opened")
			id = cmd("Open", args).then(ridFromBytes)
			await id
		},

		read: async (len) => {
			if (id === null) throw new Error("File not opened")
			const data = await cmd("Read", { id: await id, len, })
			return data.byteLength === 0 ? null : new Uint8Array(data)
		},

		close: async () => {
			if (id === null) return
			await cmd("Close", { id: await id })
		}
	}
}

type CmdWriteFileStreamHandler = {
	open: (args: Record<string, unknown>) => Promise<void>,
	write: (data: Uint8Array<ArrayBufferLike>) => Promise<void>,
	close: () => Promise<void>,
}
function resolveCmdWriteFileStream(cmdName: string): CmdWriteFileStreamHandler {
	// Tauri IPC の制約により、大きいバイトを送る際は body で、それ以外の値は headers で送信する。
	type CmdEvents = {
		Open: { in: { body: Uint8Array, args: Record<string, unknown> }, out: { id: number, supportsRawIpcRequestBody: boolean } },
		Write: { in: { body: Uint8Array | { data: string }, args: { id: number } }, out: void },
		Close: { in: { body: {}, args: { id: number } }, out: void },
	}
	type CmdType = keyof CmdEvents
	type CmdInputBody<T extends CmdType> = CmdEvents[T]["in"]["body"]
	type CmdInputArgs<T extends CmdType> = CmdEvents[T]["in"]["args"]
	type CmdOutput<T extends CmdType> = CmdEvents[T]["out"]
	function cmd<T extends CmdType>(type: T, body: CmdInputBody<T>, args: CmdInputArgs<T>): Promise<CmdOutput<T>> {
		return invoke(cmdName, body, {
			headers: {
				"tfps-cmd-type": type,
				"tfps-cmd-args": encodeURIComponent(JSON.stringify(args))
			}
		})
	}


	const PAYLOAD_FOR_CHECKING_RAW_IPC_REQUEST_BODY_SUPPORTED = new Uint8Array([0]);

	let state: Promise<{ id: number, supportsRawIpcRequestBody: boolean }> | null = null

	return {
		open: async (args) => {
			if (state !== null) throw new Error("File already opened")
			state = cmd(
				"Open",
				PAYLOAD_FOR_CHECKING_RAW_IPC_REQUEST_BODY_SUPPORTED,
				args
			)
			await state
		},

		write: async (chunk) => {
			if (state === null) throw new Error("File not opened")
			const { id, supportsRawIpcRequestBody } = await state

			if (supportsRawIpcRequestBody) {
				await cmd("Write", chunk, { id })
			}
			else {
				// IPC のリクエストで raw Body を送れない場合、
				// 大きな配列に対して非常に非効率な形式にシリアライズされる。
				// よって、まだマシな dataURL としてデータを送る。
				// Data URL を用いる理由は web API の FileReader で比較的効率的に作成できるため。
				// <https://github.com/tauri-apps/tauri/issues/10573>
				await cmd("Write", { data: await bytesToDataUrl(chunk) }, { id })
			}
		},

		close: async () => {
			if (state === null) return
			const { id } = await state
			await cmd("Close", {}, { id })
		},
	}
}

function createTextLinesReadableStream(
	handler: {
		/** null か空で EOF。 */
		read: () => Promise<Uint8Array<ArrayBuffer> | null>,
		release?: () => Promise<void>
	},
	options?: {
		fatal?: boolean,
		label?: string,
	},
	signal?: AbortSignal
): ReadableStream<{
	line: string,
	lineBreak: "\n" | "\r\n" | null
}> {

	throwIfAborted(signal)

	/*
	 * bytes は以下の形式のレコードが連続したものであり、
	 * 各レコードが分断されることはない。
	 * 
	 * - err flag (u8, 0 = ok, 1 = err)
	 * - line break type (u8, 0 = null, 1 = "\n", 2 = "\r\n")
	 * - line bytes len (u64, big endian)
	 * - line bytes (variable bytes)
	 * 
	 * err flag が 0 の場合、正常にその行が読み込まれたことを指す。
	 * この場合、line bytes には BOM 処理されたテキストが格納される。
	 * 
	 * err flag が 1 の場合、その行でエラーが発生したことを示す。
	 * この場合、line bytes には utf-8 形式のエラーメッセージが格納され、
	 * この呼び出しでの最後の行となる。
	 * 
	 * エラー発生後の呼び出しの挙動は未定義。
	 */
	const ERR_FLAG_LEN = 1;
	const LINE_BREAK_TYPE_LEN = 1;
	const LINE_LEN_LEN = 8;

	const ERR_FLAG_OFFSET = 0;
	const LINE_BREAK_TYPE_OFFSET = ERR_FLAG_OFFSET + ERR_FLAG_LEN;
	const LINE_LEN_OFFSET = LINE_BREAK_TYPE_OFFSET + LINE_BREAK_TYPE_LEN;
	const LINE_OFFSET = LINE_LEN_OFFSET + LINE_LEN_LEN;

	const LINE_BREAK_NULL = 0
	const LINE_BREAK_LF = 1
	const LINE_BREAK_CRLF = 2

	let abortListener: (() => void) | null = null
	let decoder: TextDecoder | null = null
	let buffer: Uint8Array<ArrayBuffer> | null = null

	let cleanupPromise: Promise<void> | null = null
	function cleanup(): Promise<void> {
		if (cleanupPromise === null) {
			cleanupPromise = (async () => {
				buffer = null
				decoder = null
				if (signal != null && abortListener != null) {
					signal.removeEventListener("abort", abortListener)
					abortListener = null
				}
				if (handler.release) {
					await handler.release()
				}
			})()
		}
		return cleanupPromise
	}

	// エラーはその原因となった行を読み込んだ際に発生させたいため、
	// 1回の pull では1回だけ enqueue　を行う。
	// 複数回行うとエラーが発生した行ではない箇所で read してもエラーになってしまう。
	return new ReadableStream({
		start(controller) {
			if (signal) {
				abortListener = () => {
					cleanup().catch(() => { })
					controller.error(signal.reason ?? newAbortError())
				}
				signal.addEventListener("abort", abortListener);
			}
		},

		async pull(controller) {
			try {
				throwIfAborted(signal)
				if (buffer == null || buffer.byteLength === 0) {
					buffer = await handler.read()
					throwIfAborted(signal)
				}
				if (buffer == null || buffer.byteLength === 0) {
					await cleanup()
					controller.close()
					return
				}

				if (buffer.byteLength < LINE_OFFSET) {
					throw new Error("Invalid data: Chunk ended with partial header.")
				}
				const lineLen = trySafeU64FromBytes(
					buffer.subarray(LINE_LEN_OFFSET, LINE_LEN_OFFSET + LINE_LEN_LEN),
					"bigEndian"
				)

				if (buffer.byteLength < LINE_OFFSET + lineLen) {
					throw new Error("Invalid data: Line split detected")
				}
				const lineBytes = buffer.subarray(LINE_OFFSET, LINE_OFFSET + lineLen)

				const errFlag = buffer[ERR_FLAG_OFFSET]
				if (numToFlag(errFlag)) {
					throw new Error((new TextDecoder("utf-8")).decode(lineBytes))
				}

				const lineBreakType = buffer[LINE_BREAK_TYPE_OFFSET]
				let lineBreak: "\n" | "\r\n" | null = null
				if (lineBreakType === LINE_BREAK_LF) lineBreak = "\n"
				else if (lineBreakType === LINE_BREAK_CRLF) lineBreak = "\r\n"
				else if (lineBreakType === LINE_BREAK_NULL) lineBreak = null
				else throw new Error(`Invalid lineBreakType: ${lineBreakType}`)

				if (decoder == null) {
					decoder = new TextDecoder(options?.label, {
						fatal: options?.fatal,
						ignoreBOM: true
					})
				}
				const line = decoder.decode(lineBytes)

				throwIfAborted(signal)
				controller.enqueue({ line, lineBreak })
				buffer = buffer.subarray(LINE_OFFSET + lineLen)
			}
			catch (e) {
				await cleanup().catch(() => { })
				throw e
			}
		},

		async cancel() {
			await cleanup()
		}
	})
}

function throwIfAborted(signal: AbortSignal | undefined | null) {
	if (signal?.aborted === true) {
		throw (signal?.reason ?? newAbortError())
	}
}

function newAbortError(): DOMException {
	return new DOMException("The operation was aborted.", "AbortError")
}

async function bytesToDataUrl(bytes: Uint8Array<ArrayBufferLike>): Promise<string> {
	const buffer = bytes.buffer instanceof ArrayBuffer
		? bytes as Uint8Array<ArrayBuffer>
		: new Uint8Array(bytes)

	const blob = new Blob([buffer], { type: "application/octet-stream" })
	return await blobToDataUrl(blob)
}

async function blobToDataUrl(blob: Blob): Promise<string> {
	return new Promise((resolve, reject) => {
		const reader = new FileReader()

		reader.onload = () => {
			const result = reader.result
			unsub()
			if (typeof result === "string") {
				resolve(result)
			}
			else {
				reject(new Error("FileReader result is not a string"))
			}
		}
		reader.onerror = () => {
			unsub()
			reject(reader.error ?? new Error("FileReader failed"))
		}
		reader.onabort = () => {
			unsub()
			reject(new Error("FileReader aborted"))
		}

		function unsub() {
			reader.onload = null
			reader.onerror = null
			reader.onabort = null
		}

		try {
			reader.readAsDataURL(blob)
		}
		catch (err) {
			unsub()
			reject(err)
		}
	})
}

function isNonzeroSafeInt(num: number): boolean {
	return isSafeInt(num) && num !== 0
}

function isSafeInt(num: number): boolean {
	return Number.isSafeInteger(num) && 0 <= num
}

function ridFromBytes(bytes: ArrayBufferView | ArrayBuffer): number {
	return u32FromBytes(bytes, "bigEndian")
}

function u32FromBytes(
	input: ArrayBufferView | ArrayBuffer,
	endian: "bigEndian" | "littleEndian"
): number {

	const bytes = input instanceof Uint8Array
		? input
		: input instanceof ArrayBuffer
			? new Uint8Array(input)
			: new Uint8Array(input.buffer, input.byteOffset, input.byteLength);

	if (bytes.length !== 4) {
		throw new Error("Expected 4 bytes for u32");
	}

	if (endian === "bigEndian") {
		// Big Endian: [0xAA, 0xBB, 0xCC, 0xDD] -> 0xAABBCCDD
		return ((bytes[0] << 24) | (bytes[1] << 16) | (bytes[2] << 8) | bytes[3]) >>> 0;
	}
	else {
		// Little Endian: [0xDD, 0xCC, 0xBB, 0xAA] -> 0xAABBCCDD
		return (bytes[0] | (bytes[1] << 8) | (bytes[2] << 16) | (bytes[3] << 24)) >>> 0;
	}
}

function numToFlag(flag: number): boolean {
	if (flag === 1) return true
	if (flag === 0) return false
	throw new Error("Invalid flag value")
}

function trySafeU64FromBytes(
	input: ArrayBufferView | ArrayBuffer,
	endian: "bigEndian" | "littleEndian"
): number {

	const bytes = input instanceof Uint8Array
		? input
		: input instanceof ArrayBuffer
			? new Uint8Array(input)
			: new Uint8Array(input.buffer, input.byteOffset, input.byteLength);

	if (bytes.length !== 8) {
		throw new Error("Expected 8 bytes for u64");
	}

	if (endian === "bigEndian") {
		// bytes[0]: bits 56-63 (全ビット禁止)
		// bytes[1]: bits 48-55 (上位3ビット: 53, 54, 55 が禁止)
		if (bytes[0] !== 0 || (bytes[1] & 0b1110_0000) !== 0) {
			throw new Error("u64 exceeds Number.MAX_SAFE_INTEGER");
		}

		return (
			(bytes[0] * (2 ** 56)) +
			(bytes[1] * (2 ** 48)) +
			(bytes[2] * (2 ** 40)) +
			(bytes[3] * (2 ** 32)) +
			(bytes[4] * (2 ** 24)) +
			(bytes[5] * (2 ** 16)) +
			(bytes[6] * (2 ** 8)) +
			(bytes[7])
		)
	}
	else {
		// little endian
		// bytes[7]: bits 56-63 (全ビット禁止)
		// bytes[6]: bits 48-55 (上位3ビット: 53, 54, 55 が禁止)
		if (bytes[7] !== 0 || (bytes[6] & 0b1110_0000) !== 0) {
			throw new Error("u64 exceeds Number.MAX_SAFE_INTEGER");
		}

		return (
			(bytes[0]) +
			(bytes[1] * (2 ** 8)) +
			(bytes[2] * (2 ** 16)) +
			(bytes[3] * (2 ** 24)) +
			(bytes[4] * (2 ** 32)) +
			(bytes[5] * (2 ** 40)) +
			(bytes[6] * (2 ** 48)) +
			(bytes[7] * (2 ** 56))
		)
	}
}