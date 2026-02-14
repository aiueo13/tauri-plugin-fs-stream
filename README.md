Note: **I’m using a translation tool, so there may be some inappropriate expressions.**

# Overview

This plugin provides functions that create `ReadableStream` and  `WritableStream` from a file path.

# Setup
First, install this plugin to your Tauri project:

`src-tauri/Cargo.toml`

```toml
[dependencies]
tauri-plugin-fs-stream = "=0.1.0"
```

Next, register this plugin in your Tauri project:

`src-tauri/src/lib.rs`

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs_stream::init()) // This
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Then, set the APIs and file paths that can be used from the Javascript:

`src-tauri/capabilities/*.json`
```json
{
    "permissions": [
        {
            "identifier": "fs-stream:allow-open-read-file-stream",
            "allow": ["$APPDATA/my-data/**/*"]
        },
        {
            "identifier": "fs-stream:allow-open-read-text-file-lines-stream",
            "allow": ["$APPDATA/my-data/**/*"]
        },
        {
            "identifier": "fs-stream:allow-open-write-file-stream",
            "allow": ["$APPDATA/my-data/**/*"],
            "deny": ["$APPDATA/my-data/readonly/**/*"]
        }
    ]
}
```

Finally, install the JavaScript Guest bindings using whichever JavaScript package manager you prefer:

```bash
pnpm add tauri-plugin-fs-stream-api@0.1.0 -E
# or
npm install tauri-plugin-fs-stream-api@0.1.0 --save-exact
# or
yarn add tauri-plugin-fs-stream-api@0.1.0 --exact
```

**NOTE**: Please make sure that the Rust-side `tauri-plugin-fs-stream` and the JavaScript-side `tauri-plugin-fs-stream-api` versions match exactly.

# Usage
```typescript
import { openReadFileStream, openWriteFileStream } from "tauri-plugin-fs-stream-api";

async function convertFile(
  inputPath: string,
  outputPath: string,
  convertor: TransformStream<Uint8Array<ArrayBuffer>, Uint8Array>
) {

  let input: ReadableStream<Uint8Array<ArrayBuffer>> | null = null
  let output: WritableStream<Uint8Array> | null = null
  try {
    input = await openReadFileStream(inputPath)
    output = await openWriteFileStream(outputPath)
    await input.pipeThrough(convertor).pipeTo(output)
  }
  catch (e) {
    // Ensure streams are closed
    await input?.cancel().catch(() => {})
    await output?.abort().catch(() => {})
    throw e
  }
}
```

# License
This project is licensed under either of

 * MIT license
 * Apache License (Version 2.0)

at your option.