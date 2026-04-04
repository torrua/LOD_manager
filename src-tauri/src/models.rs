use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct WordListItem {
    pub id: i64,
    pub name: String,
    pub type_name: Option<String>,
    pub def_count: i64,
}

#[derive(Serialize, Clone)]
pub struct WordDetail {
    pub id: i64,
    pub name: String,
    pub type_name: Option<String>,
    pub type_id: Option<i64>,
    pub source: Option<String>,
    pub year: Option<String>,
    pub rank: Option<String>,
    pub match_: Option<String>,
    pub origin: Option<String>,
    pub origin_x: Option<String>,
    pub notes: Option<String>,
    pub event_start_name: Option<String>,
    pub event_end_name: Option<String>,
    pub affixes: Vec<String>,
    pub spellings: Vec<String>,
    pub definitions: Vec<Definition>,
    pub used_in: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Definition {
    pub id: i64,
    pub position: i64,
    pub grammar: Option<String>,
    pub usage: Option<String>,
    pub body: String,
    pub tags: Option<String>,
}

#[derive(Deserialize)]
pub struct SaveWord {
    pub name: String,
    pub type_name: Option<String>,
    pub source: Option<String>,
    pub year: Option<String>,
    pub rank: Option<String>,
    pub match_: Option<String>,
    pub origin: Option<String>,
    pub origin_x: Option<String>,
    pub notes: Option<String>,
    pub event_start: Option<String>,
    pub event_end: Option<String>,
    pub affixes: Vec<String>,
    pub spellings: Vec<String>,
}

#[derive(Deserialize)]
pub struct SaveDefinition {
    pub grammar: Option<String>,
    pub usage: Option<String>,
    pub body: String,
    pub tags: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct EventItem {
    pub id: i64,
    pub name: String,
    pub date: Option<String>,
    pub annotation: Option<String>,
    pub suffix: Option<String>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct SaveEvent {
    pub name: String,
    pub date: Option<String>,
    pub annotation: Option<String>,
    pub suffix: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct TypeItem {
    pub id: i64,
    pub name: String,
    pub type_x: Option<String>,
    pub group_: Option<String>,
    pub word_count: i64,
}

#[derive(Deserialize)]
pub struct SaveType {
    pub name: String,
    pub type_x: Option<String>,
    pub group_: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct AuthorItem {
    pub id: i64,
    pub initials: String,
    pub full_name: Option<String>,
    pub notes: Option<String>,
    pub word_count: i64,
}

#[derive(Deserialize)]
pub struct SaveAuthor {
    pub initials: String,
    pub full_name: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize)]
pub struct ImportResult {
    pub words: usize,
    pub definitions: usize,
    pub events: usize,
    pub types: usize,
    pub authors: usize,
    pub settings: usize,
    pub errors: usize,
    pub skipped_rows: usize,
    pub messages: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct SettingItem {
    pub key: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct DbStats {
    pub db_path: String,
    pub word_count: i64,
    pub definition_count: i64,
    pub event_count: i64,
    pub type_count: i64,
    pub author_count: i64,
    pub affix_count: i64,
    pub spelling_count: i64,
    pub settings: Vec<SettingItem>,
}

// Keep AppInfo for backward compat
#[derive(Serialize)]
pub struct AppInfo {
    pub db_path: String,
    pub word_count: i64,
    pub definition_count: i64,
}

// ─── English-to-Loglan search result ─────────────────────────────────────────
#[derive(Serialize, Clone)]
pub struct ELResult {
    pub word_id: i64,
    pub word_name: String,
    pub type_name: Option<String>,
    pub grammar: Option<String>,
    pub snippet: String,  // highlighted snippet of definition body
    pub match_count: i64, // how many definitions of this word matched
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ELSearchParams {
    pub query: String,
    /// Use LIKE instead of FTS5 (fallback for databases without a built index).
    pub use_like: bool,
    /// Search only within «keyword» markers instead of full definition body.
    pub use_keywords_only: bool,
    pub limit: i64,
}
