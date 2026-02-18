# ver. 0.5
- 特定の条件における openWriteFileStream のパフォーマンスを改善。

# ver. 0.4
- ドキュメントを変更。
- openReadFileStream, openReadTextFileLinesStream の引数に options.freezeSize を追加。

# ver. 0.3
- ドキュメントを変更。
- openReadFileStream, openWriteFileStream, openReadTextFileLinesStream がファイルスキームの URL を受け入れるように変更。

# ver. 0.2
- ドキュメントを変更。
- tauri_plugin_fs_stream::{Error, Result} を非公開に変更。
- drag and drop や dialog plugin により選択されたファイルにアクセスできるように変更。
- src-tauri/tauri.conf.json　で plugins の fs-stream に require_literal_leading_dot を設定できるように変更。

# ver. 0.1
- リリース。
