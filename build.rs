#[path = "src/cmds/scope.rs"]
mod scope;

fn main() {
    tauri_plugin::Builder::new(&[
			"open_read_file_stream",
			"open_read_text_file_lines_stream",
			"open_write_file_stream",
			"close_all_file_streams",
		])
		.global_scope_schema(schemars::schema_for!(scope::ScopeSchema))
        .build();
}