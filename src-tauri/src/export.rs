//! HTML export for the Loglan Online Dictionary.
//!
//! # Performance
//! Uses 4 bulk queries (words, definitions, affixes, used-in) instead of N+1
//! per-word queries. With 10 000 words the old approach ran ~30 000 individual
//! SQL statements; the new approach runs 4 bulk queries.

use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::fmt::Write;

struct WordRow {
    id: i64,
    name: String,
    type_name: Option<String>,
    source: Option<String>,
    year: Option<String>,
    rank: Option<String>,
    match_: Option<String>,
    origin: Option<String>,
    origin_x: Option<String>,
    notes: Option<String>,
}

// Moved out of function to appease clippy `items_after_statements`.
type DefRow = (Option<String>, Option<String>, String, Option<String>);

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const SCRIPT_PREFIX: &str = r"<script>
function doSearch(q){
  q=q.toLowerCase().trim();
  var first=null;
  document.querySelectorAll('.entry').forEach(function(el){
    var name=el.dataset.name||'';
    if(!q||name.startsWith(q)){
      if(!first)first=el;
    }
  });
  if(first){
    var behavior=q.length<4?'auto':'smooth';
    first.scrollIntoView({behavior:behavior,block:'start'});
  }
}
";

const SCRIPT_SUFFIX: &str = r"
document.getElementById('lod-search').addEventListener('input',function(){doSearch(this.value);});
</script>";

fn fmt_body(s: &str) -> String {
    let s = esc(s).replace("--", "\u{2014}");
    let s = s.replace(" % ", " \u{2014} ").replace("% ", "\u{2014} ");
    let s = render_brace_spans(&s);
    render_kw_spans(&s)
}

/// Replace {text} markers with `<span class="br">` spans (bold, бордовый).
fn render_brace_spans(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 32);
    let mut chars = s.char_indices().peekable();
    while let Some((_i, c)) = chars.next() {
        if c == '{' {
            out.push_str("<span class=\"br\">");
            for (_, c2) in chars.by_ref() {
                if c2 == '}' {
                    break;
                }
                out.push(c2);
            }
            out.push_str("</span>");
        } else {
            out.push(c);
        }
    }
    out
}

/// Replace «keyword» markers with `<em class="kw">` spans.
/// Named `render_kw_spans` because it does NOT use the `regex` crate.
fn render_kw_spans(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 32);
    let mut chars = s.char_indices().peekable();
    while let Some((_i, c)) = chars.next() {
        if c == '\u{AB}' {
            out.push_str("<em class=\"kw\">");
            for (_, c2) in chars.by_ref() {
                if c2 == '\u{BB}' {
                    break;
                }
                out.push(c2);
            }
            out.push_str("</em>");
        } else {
            out.push(c);
        }
    }
    out
}

const STYLE: &str = r"<style>
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:Georgia,serif;font-size:14px;line-height:1.6;background:#faf8f2;color:#1e1a0e}
.wrap{display:flex;min-height:100vh}
.sidebar{width:200px;background:#f0ebe0;border-right:1px solid #d8d0c0;
  padding:1rem .75rem;flex-shrink:0;position:sticky;top:0;height:100vh;overflow-y:auto}
.sidebar h2{font-size:.75rem;font-weight:700;color:#7a5418;letter-spacing:.05em;margin-bottom:.6rem}
.sidebar .event-filter{font-size:.65rem;color:#6a5c48;background:#e8e0d0;padding:.2rem .5rem;border-radius:3px;margin-bottom:.5rem;text-align:center}
.alpha{display:flex;flex-wrap:wrap;gap:3px;margin-bottom:1rem;justify-content:center}
.alpha a{display:inline-flex;align-items:center;justify-content:center;width:24px;height:20px;font-size:.72rem;color:#7a5418;
  border:1px solid #d8d0c0;border-radius:3px;text-decoration:none;background:#faf8f2}
.alpha a:hover{background:#c8a050;color:#fff;border-color:#c8a050}
.search-box{width:100%;padding:.3rem .4rem;font-size:.75rem;border:1px solid #d8d0c0;
  border-radius:4px;background:#fff;margin-bottom:.5rem;font-family:inherit}
.search-box:focus{outline:none;border-color:#c8a050}
.content{flex:1;padding:1.5rem 2rem;max-width:860px}
.letter-section{margin-bottom:2rem}
.letter-head{font-size:1.4rem;font-weight:700;color:#7a5418;
  border-bottom:2px solid #d8d0c0;padding-bottom:.2rem;margin-bottom:1rem}
.entry{margin-bottom:1.1rem;padding-bottom:1rem;border-bottom:1px solid #ece5d8}
.entry:last-child{border-bottom:none}
.entry-name{font-size:1rem;font-weight:700;color:#1e1a0e;margin-bottom:.1rem}
.entry-meta{font-size:.72rem;color:#6a5c48;margin-bottom:.3rem;font-family:monospace}
.entry-notes{font-size:.7rem;color:#6a5c48;margin-bottom:.3rem;font-style:italic}
.origin{color:#5a8040;font-style:italic}
.defs{margin:.2rem 0 .2rem 1rem}
.def{margin-bottom:.25rem;font-size:.85rem}
.grammar{color:#1a6860;font-size:.78rem}
.usage{color:#7a5418;font-weight:600;font-size:.82rem}
.tags{color:#8a8070;font-size:.75rem}
em.kw{color:#1a6860;font-style:italic}
span.br{color:#800000;font-weight:700}
.used-in{font-size:.72rem;color:#6a5c48;margin-top:.3rem}
.used-in a{color:#7a5418;text-decoration:none}
.used-in a:hover{text-decoration:underline}
@media(max-width:600px){
  .wrap{flex-direction:column}
  .sidebar{width:100%;height:auto;position:static}
  .content{padding:1rem}
}
</style>";

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        params![table],
        |r| r.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}

pub fn generate_html(conn: &Connection, event_name: Option<&str>) -> rusqlite::Result<String> {
    let script = [SCRIPT_PREFIX, SCRIPT_SUFFIX].join("");
    let event_filter_label = event_name.map(|ev| format!("Event: {ev}"));

    let has_connect_authors = table_exists(conn, "connect_authors");
    let source_expr = if has_connect_authors {
        "(SELECT GROUP_CONCAT(a.abbreviation, '/') FROM connect_authors ca JOIN authors a ON a.id = ca.\"AID\" WHERE ca.\"WID\" = w.id)"
    } else {
        "NULL"
    };

    // ── 1. Load all words in ONE query ────────────────────────────────────────
    let rows: Vec<WordRow> = if let Some(ev) = event_name {
        let sql = format!(
            "SELECT w.id, w.name, t.type, {source_expr}, w.year, w.rank, w.\"match\",
                    w.origin, w.origin_x, w.notes
             FROM words w
             LEFT JOIN types t ON t.id = w.type
             WHERE (w.event_start <= (SELECT event_id FROM events WHERE name = ?1))
               AND (w.event_end IS NULL OR w.event_end > (SELECT event_id FROM events WHERE name = ?1))
             ORDER BY LOWER(w.name)"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows: Vec<WordRow> = stmt
            .query_map(params![ev], map_word_row)?
            .filter_map(std::result::Result::ok)
            .collect();
        rows
    } else {
        let sql = format!(
            "SELECT w.id, w.name, t.type, {source_expr}, w.year, w.rank, w.\"match\",
                    w.origin, w.origin_x, w.notes
             FROM words w
             LEFT JOIN types t ON t.id = w.type
             ORDER BY LOWER(w.name)"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows: Vec<WordRow> = stmt
            .query_map([], map_word_row)?
            .filter_map(std::result::Result::ok)
            .collect();
        rows
    };

    if rows.is_empty() {
        return Ok(
            "<!DOCTYPE html><html lang=\"en\"><body><p>No words found.</p></body></html>"
                .to_string(),
        );
    }

    let ids: Vec<i64> = rows.iter().map(|w| w.id).collect();

    // ── 2. Bulk-load ALL definitions for these words (1 query) ───────────────
    let mut defs_map: HashMap<i64, Vec<DefRow>> = HashMap::with_capacity(ids.len());
    {
        let mut stmt = conn.prepare(
            "SELECT word_id,
                    NULLIF(COALESCE(CAST(slots AS TEXT), '') || COALESCE(grammar_code, ''), ''),
                    usage, body, case_tags
             FROM definitions
             ORDER BY word_id, position",
        )?;
        let iter = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })?;
        for row in iter.filter_map(std::result::Result::ok) {
            defs_map
                .entry(row.0)
                .or_default()
                .push((row.1, row.2, row.3, row.4));
        }
    }

    // ── 3. Bulk-load ALL affixes for these words ─────────────────────────────
    let mut afx_map: HashMap<i64, Vec<String>> = HashMap::with_capacity(ids.len() / 4);
    if table_exists(conn, "connect_words") {
        let mut stmt = conn.prepare(
            "SELECT cw.parent_id, REPLACE(w.name, '-', '')
             FROM connect_words cw
             JOIN words w ON w.id = cw.child_id
             LEFT JOIN types t ON t.id = w.type
             WHERE t.type_x = 'Affix' OR w.name LIKE '%-' OR w.name LIKE '-%'
             ORDER BY cw.parent_id, w.id",
        )?;
        let iter = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        for (wid, afx) in iter.filter_map(std::result::Result::ok) {
            let entry = afx_map.entry(wid).or_default();
            if !entry.contains(&afx) {
                entry.push(afx);
            }
        }
    }
    if table_exists(conn, "word_affixes") {
        let mut stmt =
            conn.prepare("SELECT word_id, affix FROM word_affixes ORDER BY word_id, id")?;
        let iter = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        for (wid, afx) in iter.filter_map(std::result::Result::ok) {
            let cleaned = afx.replace('-', "");
            let entry = afx_map.entry(wid).or_default();
            if !entry.contains(&cleaned) {
                entry.push(cleaned);
            }
        }
    }

    // ── 4. Build "used in" map from connect_words and word_usage ─────────────
    let mut used_map: HashMap<i64, Vec<String>> = HashMap::new();
    if table_exists(conn, "connect_words") {
        let mut stmt = conn.prepare(
            "SELECT cw.parent_id, w.name
             FROM connect_words cw
             JOIN words w ON w.id = cw.child_id
             LEFT JOIN types t ON t.id = w.type
             WHERE t.\"group\" = 'Cpx' OR t.type IN ('2-Cpx', '3-Cpx', 'C-Cpx')
             ORDER BY cw.parent_id, LOWER(w.name)",
        )?;
        let iter = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        for (wid, name) in iter.filter_map(std::result::Result::ok) {
            let entry = used_map.entry(wid).or_default();
            if !entry.contains(&name) {
                entry.push(name);
            }
        }
    }
    if table_exists(conn, "word_usage") {
        let mut stmt =
            conn.prepare("SELECT word_id, used_in_word FROM word_usage ORDER BY word_id")?;
        let iter = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        for (wid, name) in iter.filter_map(std::result::Result::ok) {
            let entry = used_map.entry(wid).or_default();
            if !entry.contains(&name) {
                entry.push(name);
            }
        }
    }

    // ── 5. Build HTML string ──────────────────────────────────────────────────
    let letters: Vec<char> = rows
        .iter()
        .filter_map(|w| w.name.chars().next().map(|c| c.to_ascii_uppercase()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    let mut html = String::with_capacity(rows.len() * 400);

    let title = match event_name {
        Some(ev) => format!("LOD — {}", esc(ev)),
        None => "Loglan Online Dictionary".to_string(),
    };
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n");
    let _ = writeln!(html, "<title>{title}</title>");
    html.push_str(STYLE);
    html.push_str("</head>\n<body>\n<div class=\"wrap\">\n<nav class=\"sidebar\">\n");
    html.push_str("<h2>LOD</h2>\n");
    if let Some(ref label) = event_filter_label {
        let _ = writeln!(html, "<div class=\"event-filter\">{}</div>", esc(label));
    }
    html.push_str(
        "<input id=\"lod-search\" class=\"search-box\" type=\"text\" placeholder=\"Search…\">\n",
    );
    html.push_str("<div class=\"alpha\">\n");
    for c in &letters {
        let _ = writeln!(html, "<a href=\"#L{c}\">{c}</a>");
    }
    html.push_str("</div>\n</nav>\n<main class=\"content\">\n");

    let empty_strs: Vec<String> = Vec::new();
    let empty_defs: Vec<DefRow> = Vec::new();
    let mut cur_letter = '\0';

    for w in &rows {
        let first = w
            .name
            .chars()
            .next()
            .map_or('?', |c| c.to_ascii_uppercase());
        if first != cur_letter {
            if cur_letter != '\0' {
                html.push_str("</div></div>\n");
            }
            cur_letter = first;
            let _ = write!(
                html,
                "<div class=\"letter-section\" id=\"L{first}\">\
                 \n<div class=\"letter-head\">{first}</div>\n<div>\n"
            );
        }

        let affixes = afx_map.get(&w.id).unwrap_or(&empty_strs);
        let used_in = used_map.get(&w.id).unwrap_or(&empty_strs);
        let defs = defs_map.get(&w.id).unwrap_or(&empty_defs);

        let _ = writeln!(
            html,
            "<div class=\"entry\" data-name=\"{}\">",
            w.name.to_lowercase()
        );
        let _ = write!(html, "<div class=\"entry-name\">{}", esc(&w.name));
        for a in affixes {
            let _ = write!(
                html,
                " <span style=\"font-size:.7rem;color:#5a8040;\
                 border:1px solid #5a8040;border-radius:2px;padding:0 3px\">{}</span>",
                esc(a)
            );
        }
        html.push_str("</div>\n");

        // Meta line
        let mut meta: Vec<String> = Vec::new();
        if let Some(ref origin) = w.origin {
            let ox = w.origin_x.as_deref().unwrap_or("");
            let ox_part = if ox.is_empty() {
                String::new()
            } else {
                format!(" = {}", esc(ox))
            };
            meta.push(format!(
                "<span class=\"origin\">&lt;{}{}&gt;</span>",
                esc(origin),
                ox_part
            ));
        }
        if let Some(ref m) = w.match_ {
            meta.push(esc(m));
        }
        if let Some(ref t) = w.type_name {
            meta.push(esc(t));
        }
        if let Some(ref s) = w.source {
            meta.push(esc(s));
        }
        if let Some(ref y) = w.year {
            meta.push(esc(y));
        }
        if let Some(ref r) = w.rank {
            meta.push(esc(r));
        }
        if !meta.is_empty() {
            let _ = writeln!(html, "<div class=\"entry-meta\">{}</div>", meta.join(" · "));
        }

        // Notes
        if let Some(ref notes) = w.notes {
            let _ = writeln!(html, "<div class=\"entry-notes\">{}</div>", esc(notes));
        }

        // Definitions
        if !defs.is_empty() {
            html.push_str("<div class=\"defs\">\n");
            for (grammar, usage, body, tags) in defs {
                html.push_str("<div class=\"def\">");
                if let Some(u) = usage {
                    let u2 = u.replace('%', &esc(&w.name));
                    let _ = write!(html, "<span class=\"usage\">{} </span>", esc(&u2));
                }
                if let Some(g) = grammar {
                    let _ = write!(html, "<span class=\"grammar\">({})</span> ", esc(g));
                }
                html.push_str(&fmt_body(body));
                if let Some(t) = tags {
                    let _ = write!(html, " <span class=\"tags\">[{}]</span>", esc(t));
                }
                html.push_str("</div>\n");
            }
            html.push_str("</div>\n");
        }

        // Used in
        if !used_in.is_empty() {
            html.push_str("<div class=\"used-in\">Used in: ");
            for (i, u) in used_in.iter().enumerate() {
                if i > 0 {
                    html.push_str("; ");
                }
                let eu = esc(u);
                let je = esc(&u.to_lowercase());
                let _ = write!(
                    html,
                    "<a href=\"#\" onclick=\"\
                     var s=document.getElementById('lod-search');\
                     s['value']='{je}';\
                     doSearch('{je}');return false\">{eu}</a>"
                );
            }
            html.push_str("</div>\n");
        }

        html.push_str("</div>\n"); // .entry
    }

    if cur_letter != '\0' {
        html.push_str("</div></div>\n");
    }
    html.push_str("</main>\n</div>\n");
    html.push_str(&script);
    html.push_str("\n</body>\n</html>\n");
    Ok(html)
}

#[inline]
fn map_word_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<WordRow> {
    let raw_source: Option<String> = r.get(3)?;
    let raw_year: Option<String> = r.get(4)?;
    let raw_rank: Option<String> = r.get(5)?;
    let raw_notes: Option<String> = r.get(9)?;
    let (source, year, rank, notes) =
        crate::db::normalize_word_fields(raw_source.as_deref(), raw_year, raw_rank, raw_notes);
    Ok(WordRow {
        id: r.get(0)?,
        name: r.get(1)?,
        type_name: r.get(2)?,
        source,
        year,
        rank,
        match_: r.get(6)?,
        origin: r.get(7)?,
        origin_x: r.get(8)?,
        notes,
    })
}

/// Write the generated HTML directly to a file path.
/// Uses `std::fs::write` to bypass the Tauri FS plugin permission layer,
/// which is intentional — the path was chosen by the user via a save dialog.
pub fn write_html_to_file(
    conn: &Connection,
    path: &str,
    event_name: Option<&str>,
) -> rusqlite::Result<()> {
    let html = generate_html(conn, event_name)?;
    std::fs::write(path, html.as_bytes())
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
}
