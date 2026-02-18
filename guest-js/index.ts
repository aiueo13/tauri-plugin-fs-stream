import { invoke } from '@tauri-apps/api/core'
import { BaseDirectory } from '@tauri-apps/api/path'


export type OpenReadFileStreamOptions = {

	/**
	 * The buffer size, in bytes, used when sending data from the backend to the frontend.
	 * 
	 * IPC calls are relatively expensive, 
	 * so larger buffer sizes are generally more efficient. 
	 * But if it is too large, the UI may freeze or run out of memory.
	 * 
	 * Defaults to `524288` (512 KiB).
	 */
	bufferByteLength?: number,

	/**
	 * Indicates whether to limit the read stream to the file's size at the moment of opening.
	 * 
	 * - `true`: The stream acts as a fixed snapshot. It stops reading exactly when the initial file size is reached. This is a safety mechanism to prevent infinite loops if the stream is piped back into the same file (read/write cycle).
	 * - `false`: The stream reads until the underlying file returns EOF. This allows reading data appended to the file while the stream is open, but is unsafe for self-copying operations.
	 * 
	 * Defaults to `true`.
	 */
	freezeSize?: boolean,

	/**
	 * Base directory for `path`.
	 */
	baseDir?: BaseDirectory
}

/**
 * Opens the file with read-only mode and resolves to a `ReadableStream`.  
 * 
 * The returned `ReadableStream` must always be released by the caller.
 * Failure to do so may cause file resource leaks.
 * The returned ReadableStream is released in the following cases:
 * - When the ReadableStream or its Reader is canceled. 
 * - When the ReadableStream's Reader has been fully read. 
 * - When the ReadableStream's Reader's read operation ends with an error. 
 * 
 * These releases may be performed multiple times without issue.
 * 
 * @param path - The file path or file scheme URL to read. 
 * @param options - Optional settings: `bufferByteLength`, `freezeSize`, `baseDir`. See `OpenReadFileStreamOptions` for detailed descriptions of each item.
 * 
 * @returns A Promise that resolves to a `ReadableStream<Uint8Array<ArrayBuffer>>` backed by the file opened in read-only mode. This stream has a one-to-one correspondence with the OS handle (file descriptor on Unix or file handle on Windows).
 */
export async function openReadFileStream(
	path: string | URL,
	options?: OpenReadFileStreamOptions
): Promise<ReadableStream<Uint8Array<ArrayBuffer>>> {

	const bufferByteLength = mapBufferByteLengthForInput(options?.bufferByteLength)
	const freezeSize = options?.freezeSize ?? true
	const { open, read, close } = await resolveReadFileStreamEvents(
		"plugin:fs-stream|open_read_file_stream",
		mapFsPathForInput(path),
		{ baseDir: options?.baseDir, freezeSize }
	)

	try {
		await open()
		return createReadableStream({
			read: () => read(bufferByteLength),
			release: close
		})
	}
	catch (e) {
		await close().catch(() => { })
		throw e
	}
}


export type OpenReadTextFileLinesStreamOptions = {

	/**
	 * Text encoding label for decoder, such as `"utf-8"`, `"shift_jis"`, `"iso-8859-2"`. 
	 *  
	 * This is passed to [`TextDecoder constructor`](https://developer.mozilla.org/en-US/docs/Web/API/TextDecoder/TextDecoder).
	 * See: [the available encodings](https://developer.mozilla.org/ja/docs/Web/API/Encoding_API/Encodings).
	 * 
	 * Defaults to `"utf-8"`.
	 */
	encoding?: string,

	/**
	 * Indicates whether decoding errors should be treated as fatal.
	 *
	 * - `false`: Invalid byte sequences are replaced with U+FFFD (`�`) and decoding continues.
	 * - `true`: A `TypeError` is thrown when an invalid byte sequence is encountered.
	 *
	 * This is passed to [`TextDecoder constructor`](https://developer.mozilla.org/en-US/docs/Web/API/TextDecoder/TextDecoder).
	 *
	 * Defaults to `false`.
	 */
	fatal?: boolean,

	/**
	 * Indicates whether to ignore a leading BOM (Byte Order Mark).
	 *
	 * - `false`: A leading BOM is automatically stripped from the decoded result.
	 * - `true`: A leading BOM is preserved and treated as a normal character.
	 *
	 * Defaults to `false`.
	 */
	ignoreBOM?: boolean,

	/**
	 * The buffer size, in bytes, used when sending data from the backend to the frontend.
	 * 
	 * IPC calls are relatively expensive, 
	 * so larger buffer sizes are generally more efficient. 
	 * But if it is too large, the UI may freeze or run out of memory.
	 * 
	 * This value is not guaranteed to be strictly respected. 
	 * If a single line exceeds this size, 
	 * more bytes may be sent in a single IPC transmission.
	 * 
	 * Defaults to `524288` (512 KiB).
	 */
	bufferByteLength?: number,

	/**
	 * The maximum byte length of a line before decoding, excluding line break characters and an initial BOM (if present). 
	 * If a line exceeds this limit, an error is thrown. 
	 * This prevents OOM errors when reading minified files or binaries.
	 * 
	 * Defaults to `0` (unlimited).
	 */
	maxLineByteLength?: number,

	/**
	 * Indicates whether to limit the read stream to the file's size at the moment of opening.
	 * 
	 * - `true`: The stream acts as a fixed snapshot. It stops reading exactly when the initial file size is reached. This is a safety mechanism to prevent infinite loops if the stream is piped back into the same file (read/write cycle).
	 * - `false`: The stream reads until the underlying file returns EOF. This allows reading data appended to the file while the stream is open, but is unsafe for self-copying operations.
	 * 
	 * Defaults to `true`.
	 */
	freezeSize?: boolean,

	/**
	 * Base directory for `path`.
	 */
	baseDir?: BaseDirectory,
}

export type OpenReadTextFileLinesStreamItem = {

	/**
	 * A text of the current line, excluding line break characters.
	 * 
	 * If you need it, use `lineBreak`.
	 */
	line: string,

	/**
	 * Line break characters used at the end of the current line.  
	 * One of: `"\n"`, `"\r\n"`, `null`.
	 * 
	 * This value is `null`
	 * if the current line is last and the file does not end with a line break.
	 */
	lineBreak: "\n" | "\r\n" | null
}

/**
 * Opens the file with read-only mode and resolves to a `ReadableStream` of text lines. 
 *  
 * The stream yields decoded text line by line.   
 * See: `OpenReadTextFileLinesStreamItem`.
 * 
 * The returned `ReadableStream` must always be released by the caller.
 * Failure to do so may cause file resource leaks.
 * The returned ReadableStream is released in the following cases:
 * - When the ReadableStream or its Reader is canceled. 
 * - When the ReadableStream's Reader has been fully read. 
 * - When the ReadableStream's Reader's read operation ends with an error. 
 * 
 * These releases may be performed multiple times without issue.
 * 
 * @param path - The file path or file scheme URL to read. 
 * @param options - Optional settings: `encoding`, `fatal`, `ignoreBOM`, `maxLineByteLength`, `bufferByteLength`, `freezeSize`, `baseDir`. See `OpenReadTextFileLinesStreamOptions` for detailed descriptions of each item.
 * 
 * @returns A Promise that resolves to a `ReadableStream<OpenReadTextFileLinesStreamItem>` backed by the file opened in read-only mode. This stream has a one-to-one correspondence with the OS handle (file descriptor on Unix or file handle on Windows).
 */
export async function openReadTextFileLinesStream(
	path: string | URL,
	options?: OpenReadTextFileLinesStreamOptions,
): Promise<ReadableStream<OpenReadTextFileLinesStreamItem>> {

	const maxLineByteLength = mapMaxLineByteLength(options?.maxLineByteLength)
	const bufferSize = mapBufferByteLengthForInput(options?.bufferByteLength)
	const label = mapEncodingLabelForInput(options?.encoding)
	const fatal = options?.fatal ?? false
	const ignoreBOM = options?.ignoreBOM ?? false
	const freezeSize = options?.freezeSize ?? true
	const { open, read, close } = await resolveReadFileStreamEvents(
		"plugin:fs-stream|open_read_text_file_lines_stream",
		mapFsPathForInput(path),
		{ baseDir: options?.baseDir, freezeSize }
	)

	try {
		await open({ label, maxLineByteLength, ignoreBOM })
		return await createTextLinesReadableStream(
			{
				read: () => read(bufferSize),
				release: close
			},
			{ label, fatal }
		)
	}
	catch (e) {
		await close().catch(() => { })
		throw e
	}
}


export type OpenWriteFileStreamOptions = {

	/**
	 * The buffer size, in bytes, used when sending data from the frontend to the backend.
	 * 
	 * IPC calls are relatively expensive, 
	 * so larger buffer sizes are generally more efficient. 
	 * But if it is too large, the UI may freeze or run out of memory.
	 * 
	 * Defaults to `524288` (512 KiB).
	 */
	bufferByteLength?: number,

	/**
	 * Indicates whether the data should be appended from the end of the file instead of overwriting the existing content.
	 * 
	 * Defaults to `false`.
	 */
	append?: boolean,

	/**
	 * Indicates whether a new file should be created if it does not exist.
	 * 
	 * Defaults to `true`.
	 */
	create?: boolean,

	/**
	 * Indicates whether a new file must be created. 
	 * In other words, whether an error should be raised if the file already exists.
	 * 
	 * Defaults to `false`.
	 */
	createNew?: boolean,

	/**
	 * The file mode bits for creating a new file.
	 * Ignored on Windows.
	 * 
	 * See: <https://doc.rust-lang.org/std/os/unix/fs/trait.OpenOptionsExt.html#tymethod.mode>
	 */
	mode?: number,

	/**
	 * Base directory for `path`.
	 */
	baseDir?: BaseDirectory
}

/**
 * Opens the file with write-only mode and resolves to a `WritableStream`.  
 * 
 * The returned `WritableStream` must always be released by the caller.
 * Failure to do so may cause file resource leaks.
 * The returned WritableStream is released in the following cases:
 * - When the WritableStream or its Writer is closed. 
 * - When the WritableStream or its Writer is aborted. 
 * - When the WritableStream's Writer's write operation ends with an error. 
 * 
 * These releases may be performed multiple times without issue.
 * 
 * @param path - The file path or file scheme URL write to. 
 * @param options - Optional settings: `bufferByteLength`, `append`, `create`, `createNew`, `mode`, `baseDir`. See `OpenWriteFileStreamOptions` for detailed descriptions of each item.
 * 
 * @returns A Promise that resolves to a `WritableStream<Uint8Array<ArrayBufferLike>>` backed by the file opened in write-only mode. This stream has a one-to-one correspondence with the OS handle (file descriptor on Unix or file handle on Windows).
 */
export async function openWriteFileStream(
	path: string | URL,
	options?: OpenWriteFileStreamOptions
): Promise<WritableStream<Uint8Array<ArrayBufferLike>>> {

	const bufferByteLength = mapBufferByteLengthForInput(options?.bufferByteLength)
	const fileOptions = {
		append: options?.append ?? false,
		create: options?.create ?? true,
		createNew: options?.createNew ?? false,
		mode: options?.mode,
		baseDir: options?.baseDir
	}
	const { open, write, close } = await resolveWriteFileStreamEvents(
		"plugin:fs-stream|open_write_file_stream",
		mapFsPathForInput(path),
		fileOptions,
	)

	try {
		await open()
		return createBufferedWritableStream(bufferByteLength, {
			write,
			release: close
		})
	}
	catch (e) {
		await close().catch(() => { })
		throw e
	}
}


/**
 * Forcibly disposes of all file streams.
 *
 * All backend file resources owned by `ReadableStream` and `WritableStream` instances 
 * created by this plugin are detached from the frontend and released.
 * 
 * After this operation, any read or write operation on existing streams will result in an error, 
 * except for buffering on the frontend.
 */
export async function closeAllFileStreams(): Promise<void> {
	await invoke("plugin:fs-stream|close_all_file_streams")
}


/** 512 KiB */
const DEFAULT_BUFFER_SIZE_FOR_IPC = 512 * 1024;

function mapBufferByteLengthForInput(s?: number): number {
	const bufferSize = s ?? DEFAULT_BUFFER_SIZE_FOR_IPC
	if (!isNonzeroSafeInt(bufferSize)) {
		throw new Error(`Invalid bufferByteLength: expected a non-zero safe unsigned integer (1..Number.MAX_SAFE_INTEGER), got ${bufferSize}`)
	}
	return bufferSize
}

function mapEncodingLabelForInput(label?: string): string {
	try {
		return (new TextDecoder(label)).encoding
	}
	catch {
		throw new RangeError(`Bad encoding label: ${label}`)
	}
}

function mapMaxLineByteLength(s?: number): number {
	if (s == null) return 0

	if (!Number.isSafeInteger(s) || s < 0) {
		throw new Error(`Invalid maxLineByteLength: expected a safe unsigned integer, got ${s}`);
	}

	return s
}

function mapFsPathForInput(path: string | URL): string {
	return path instanceof URL ? path.toString() : path
}

type ReadFileStreamEvents = {
	open: (options?: Record<any, any>) => Promise<void>
	read: (len: number, options?: Record<any, any>) => Promise<Uint8Array<ArrayBuffer> | null>,
	close: (options?: Record<any, any>) => Promise<void>,
}
async function resolveReadFileStreamEvents(
	cmd: string,
	path: string,
	options: {
		baseDir?: BaseDirectory,
		freezeSize: boolean
	}
): Promise<ReadFileStreamEvents> {

	type CmdEvents = {
		Open: { path: string, baseDir?: BaseDirectory, freezeSize: boolean },
		Read: { id: number, len: number },
		Close: { id: number },
	}
	type CmdType = keyof CmdEvents
	type CmdInput<T extends CmdType> = CmdEvents[T]
	function dispatch<T extends CmdType>(type: T, input: CmdInput<T>): Promise<ArrayBuffer> {
		return invoke(cmd, { event: { type, ...input } })
	}


	let id: number | null = null

	return {
		open: async (ops) => {
			if (id !== null) throw new Error("File already opened")
			const idBytes = await dispatch("Open", {
				...ops,
				path,
				freezeSize: options.freezeSize,
				baseDir: options.baseDir
			})
			id = ridFromBytes(idBytes)
		},

		read: async (len, ops) => {
			if (id === null) throw new Error("File not opened")
			const data = await dispatch("Read", { ...ops, id, len, })
			return data.byteLength === 0 ? null : new Uint8Array(data)
		},

		close: async (ops) => {
			if (id === null) return
			await dispatch("Close", { ...ops, id })
		}
	}
}

type WriteFileStreamEvents = {
	open: () => Promise<void>,
	write: (data: Uint8Array<ArrayBufferLike>) => Promise<void>,
	close: () => Promise<void>,
}
async function resolveWriteFileStreamEvents(
	cmd: string,
	path: string,
	options: {
		append: boolean,
		create: boolean,
		createNew: boolean,
		mode?: number,
		baseDir?: BaseDirectory
	}
): Promise<WriteFileStreamEvents> {

	type CmdEvents = {
		Open: { body: Uint8Array, headers: { path: string, options: string }, out: { id: number, supportsRawIpcRequestBody: boolean } },
		Write: { body: Uint8Array | { data: string }, headers: { id: string }, out: void },
		Close: { body: {}, headers: { id: string }, out: void },
	}
	type CmdType = keyof CmdEvents
	type CmdInputBody<T extends CmdType> = CmdEvents[T]["body"]
	type CmdInputHeaders<T extends CmdType> = CmdEvents[T]["headers"]
	type CmdOutput<T extends CmdType> = CmdEvents[T]["out"]
	function dispatch<T extends CmdType>(type: T, body: CmdInputBody<T>, headers: CmdInputHeaders<T>): Promise<CmdOutput<T>> {
		return invoke(cmd, body, { headers: { eventType: type, ...headers } })
	}


	const PAYLOAD_FOR_CHCKING_RAW_IPC_REQUEST_BODY_SUPPROTED = new Uint8Array([0]);

	let id: string | null = null
	let supportsRawIpcRequestBody: boolean | null = null

	return {
		open: async () => {
			if (id !== null) throw new Error("File already opened")

			const res = await dispatch("Open",
				PAYLOAD_FOR_CHCKING_RAW_IPC_REQUEST_BODY_SUPPROTED,
				{
					path: encodeURIComponent(path),
					options: encodeURIComponent(JSON.stringify(options))
				}
			)
			
			supportsRawIpcRequestBody = res.supportsRawIpcRequestBody
			id = res.id.toString()
		},

		write: async (chunk) => {
			if (id === null) throw new Error("File not opened")
			if (supportsRawIpcRequestBody === null) throw new Error("Missing value: supportsRawIpcRequestBody")

			if (supportsRawIpcRequestBody) {
				await dispatch("Write", chunk, { id })
			}
			// IPC のリクエストで raw Body を送れない場合、
			// 大きな配列に対して非常に非効率な形式にシリアライズされる。
			// よって、まだマシな dataURL としてデータを送る。
			// Data URL を用いる理由は web API の FileReader で比較的効率的に作成できるため。
			// <https://github.com/tauri-apps/tauri/issues/10573>
			else {
				await dispatch("Write", { data: await bytesToDataUrl(chunk) }, { id })
			}
		},

		close: async () => {
			if (id === null) return
			await dispatch("Close", {}, { id })
		},
	}
}

let _isReadableByteStreamAvailable: boolean | null = null
function isReadableByteStreamAvailable() {
	if (_isReadableByteStreamAvailable === null) {
		try {
			new ReadableStream({ type: "bytes" })
			_isReadableByteStreamAvailable = true
		}
		catch {
			_isReadableByteStreamAvailable = false
		}
	}

	return _isReadableByteStreamAvailable
}

async function createTextLinesReadableStream(
	handler: {
		/** null か空で EOF。 */
		read: () => Promise<Uint8Array<ArrayBuffer> | null>,
		release?: () => Promise<void>
	},
	options?: {
		fatal?: boolean,
		label?: string,
	}
): Promise<ReadableStream<{
	line: string,
	lineBreak: "\n" | "\r\n" | null
}>> {

	let releasePromise: Promise<void> | null = null
	const releaseOnce = () => {
		if (!releasePromise) {
			releasePromise = (handler.release ?? (async () => { }))()
		}
		return releasePromise
	}

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

	let decoder: TextDecoder | null = null
	let buffer: Uint8Array<ArrayBuffer> | null = null

	// エラーはその原因となった行を読み込んだ際に発生させたいため、
	// 1回の pull では1回だけ enqueue　を行う。
	// 複数回行うとエラーが発生した行ではない箇所で read してもエラーになってしまう。
	return new ReadableStream({
		async pull(controller) {
			try {
				if (buffer == null || buffer.byteLength === 0) {
					buffer = await handler.read()
				}
				if (buffer == null || buffer.byteLength === 0) {
					decoder = null
					buffer = null
					await releaseOnce()
					controller.close()
					return
				}

				if (buffer.byteLength < LINE_OFFSET) {
					throw new Error(`Invalid data: Chunk ended with partial header. (${buffer.byteLength} bytes remained)`)
				}
				const lineLen = trySafeU64FromBytes(
					buffer.subarray(LINE_LEN_OFFSET, LINE_LEN_OFFSET + LINE_LEN_LEN),
					"bigEndian"
				)

				if (buffer.byteLength < LINE_OFFSET + lineLen) {
					throw new Error(`Invalid data: Line split detected. Expected ${lineLen} bytes body, but only ${buffer.byteLength - LINE_OFFSET} bytes remained in chunk.`)
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

				controller.enqueue({ line, lineBreak })
				buffer = buffer.subarray(LINE_OFFSET + lineLen)
			}
			catch (e) {
				decoder = null
				buffer = null
				await releaseOnce().catch(() => { })
				throw e
			}
		},

		async cancel() {
			decoder = null
			buffer = null
			await releaseOnce()
		}
	})
}

async function createReadableStream(
	handler: {
		/** null または空配列で EOF */
		read: () => Promise<Uint8Array<ArrayBuffer> | null>,
		release?: () => Promise<void>
	},
): Promise<ReadableStream<Uint8Array<ArrayBuffer>>> {

	let releasePromise: Promise<void> | null = null
	const releaseOnce = () => {
		if (!releasePromise) {
			releasePromise = (handler.release ?? (async () => { }))()
		}
		return releasePromise
	}

	if (!isReadableByteStreamAvailable()) {
		return new ReadableStream({
			async pull(controller) {
				try {
					const data = await handler.read()
					if (data == null || data.byteLength === 0) {
						await releaseOnce()
						controller.close()
						return
					}

					controller.enqueue(data)
				}
				catch (e) {
					await releaseOnce().catch(() => { })
					throw e
				}
			},

			async cancel() {
				await releaseOnce()
			}
		})
	}

	let buffer: Uint8Array<ArrayBuffer> | null = null

	// autoAllocateChunkSize を指定すると stream.getReader() でも byob が使われるようになるが、
	// この実装で byob を用いてもコピーが増えるだけで恩恵が少ないため指定しない。
	// また type: "bytes" で strategy を指定すると (正確には size を定義すると) エラーになる点にも注意。
	return new ReadableStream({
		type: "bytes",

		async pull(controller) {
			try {
				if (buffer == null || buffer.byteLength === 0) {
					buffer = await handler.read()
				}
				if (buffer == null || buffer.byteLength === 0) {
					buffer = null
					await releaseOnce()

					// byobRequest がある場合、respond を呼ばないと promise　が解決されない。
					// controller.close() の後だと respond(0) を読んでもエラーにはならない。
					// https://github.com/whatwg/streams/issues/1170
					controller.close()
					controller.byobRequest?.respond(0)
					return
				}

				const byob = controller.byobRequest
				// byobRequest がある場合、respond を呼ばないと promise　が解決されないことに注意
				if (byob != null) {
					// respond する前なので null にならない
					const v = byob.view!!
					const view = new Uint8Array(v.buffer, v.byteOffset, v.byteLength)
					const nread = Math.min(buffer.byteLength, view.byteLength)

					view.set(buffer.subarray(0, nread))
					buffer = buffer.subarray(nread)
					byob.respond(nread)
				}
				else {
					controller.enqueue(buffer)
					buffer = null
				}
			}
			catch (e) {
				buffer = null
				await releaseOnce().catch(() => { })

				// byobRequest が存在する場合、controller.close() を呼んだだけでは
				// Promise は解決されず、respond() も呼ぶ必要がある。
				// controller.error() も同様の挙動になる可能性がある。(要検証)
				// 少なくとも throw すれば Promise は解決されるため、現状はこの実装とする。
				throw e
			}
		},

		async cancel() {
			buffer = null
			await releaseOnce()
		}
	})
}

/**
 * chunk はクロージャーの中でのみ用いるべきであり、それ以降は参照すべきでない。
 * 必要な場合はコピーしてから用いる必要がある。
 */
async function createBufferedWritableStream(
	bufferSize: number,
	handler: {
		write: (chunk: Uint8Array<ArrayBuffer>) => Promise<void>,
		release?: () => Promise<void>
	},
): Promise<WritableStream<Uint8Array<ArrayBufferLike>>> {

	if (!Number.isSafeInteger(bufferSize) || bufferSize <= 0) {
		throw new Error("bufferSize must be a positive safe integer")
	}

	let releasePromise: Promise<void> | null = null
	const releaseOnce = () => {
		if (!releasePromise) {
			releasePromise = (handler.release ?? (async () => { }))()
		}
		return releasePromise
	}

	let buffer: Uint8Array<ArrayBuffer> | null = new Uint8Array(bufferSize)
	let bufferOffset = 0;

	return new WritableStream<Uint8Array<ArrayBufferLike>>({
		async write(src) {
			try {
				if (buffer == null) throw new Error("Buffer missing")

				let srcOffset = 0;

				while (srcOffset < src.byteLength) {
					const n = Math.min(bufferSize - bufferOffset, src.byteLength - srcOffset)
					buffer.set(src.subarray(srcOffset, srcOffset + n), bufferOffset)
					bufferOffset += n
					srcOffset += n

					if (bufferOffset === bufferSize) {
						await handler.write(buffer)
						bufferOffset = 0
					}
				}
			}
			catch (e) {
				buffer = null
				await releaseOnce().catch(() => { })
				throw e
			}
		},

		async close() {
			try {
				if (0 < bufferOffset && buffer != null) {
					await handler.write(buffer.subarray(0, bufferOffset))
				}
			}
			finally {
				buffer = null
				await releaseOnce()
			}
		},

		async abort() {
			buffer = null
			await releaseOnce()
		}
	})
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
	return Number.isSafeInteger(num) && 0 <= num && num <= Number.MAX_SAFE_INTEGER
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
		throw new Error(`Expected 4 bytes for u32, got ${bytes.length}`);
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
	throw new Error(`Invalid flag value: ${flag}`)
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
		throw new Error(`Expected 8 bytes for u64, got ${bytes.length}`);
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