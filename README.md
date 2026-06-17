Note: **I’m using a translation tool, so there may be some inappropriate expressions.**

# Overview

This plugin provides APIs that create the WebAPI `ReadableStream` and  `WritableStream` from a file path.

# Setup
First, install this plugin to your Tauri project:

`src-tauri/Cargo.toml`

```toml
[dependencies]
tauri-plugin-fs-stream = "=2.0.1"
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
            "identifier": "fs-stream:allow-open-write-file-stream",
            "allow": ["$APPDATA/my-data/**/*"],
            "deny": ["$APPDATA/my-data/readonly/**/*"]
        }
    ]
}
```

Finally, install the JavaScript Guest bindings using whichever JavaScript package manager you prefer:

```bash
pnpm add tauri-plugin-fs-stream-api@2.0.1 -E
# or
npm install tauri-plugin-fs-stream-api@2.0.1 --save-exact
# or
yarn add tauri-plugin-fs-stream-api@2.0.1 --exact
```

**NOTE**: Please make sure that the Rust-side `tauri-plugin-fs-stream` and the JavaScript-side `tauri-plugin-fs-stream-api` versions match exactly.

[![crates.io](https://img.shields.io/crates/v/tauri-plugin-fs-stream.svg?color=yellow)](https://crates.io/crates/tauri-plugin-fs-stream) [![npm version](https://img.shields.io/npm/v/tauri-plugin-fs-stream-api.svg?color=red)](https://www.npmjs.com/package/tauri-plugin-fs-stream-api)


# API
This plugin provides the following commands:
- `openReadFileStream`
- `openReadTextFileLinesStream`
- `openWriteFileStream`
- `closeAllFileStreams`
- `countAllFileStreams`

# Example
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
    await input?.cancel().catch(() => {})
    await output?.abort().catch(() => {})
    throw e
  }
}
```

# File Access
Access control for file paths follows [the same model as the fs plugin](https://v2.tauri.app/reference/javascript/fs/#security).

This plugin prevents path traversal and can access only the paths explicitly declared in the capability file. 

An exception applies to files that the user explicitly selects through drag and drop or via  [the dialog plugin](https://v2.tauri.app/plugin/dialog/). Such files are accessible even if they are not declared in the capability configuration. And these permissions can be persisted using [the persisted scope plugin](https://v2.tauri.app/plugin/persisted-scope/), allowing access to remain available across application restarts. Note that to use these features, the fs plugin must be [set up in your Tauri project](https://v2.tauri.app/plugin/file-system/#setup). 

# Optional Settings
As with the fs plugin, you can configure `plugins.fs-stream.requireLiteralLeadingDot` in `src-tauri/tauri.conf.json`.

You can also use the `"fs-stream:scope"` permission in the capability file to define allowed and denied paths for all commands.

# License
This project is licensed under either of

 * MIT license
 * Apache License (Version 2.0)

at your option.