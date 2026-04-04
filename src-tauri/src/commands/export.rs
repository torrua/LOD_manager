//! HTML export commands.

use super::{Db, Res, with_db};
use crate::export;

#[tauri::command]
pub fn export_html(state: Db, event_name: Option<String>, wildcard: bool) -> Res<String> {
    with_db(&state, |conn| {
        export::generate_html(conn, event_name.as_deref(), wildcard)
    })
}

#[tauri::command]
pub fn export_html_to_file(
    state: Db,
    path: String,
    event_name: Option<String>,
    wildcard: bool,
) -> Res<()> {
    with_db(&state, |conn| {
        export::write_html_to_file(conn, &path, event_name.as_deref(), wildcard)
    })
}
