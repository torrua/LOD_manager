use rusqlite::{params, Connection};

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn fmt_body(s: &str) -> String {
    let s = esc(s).replace("--", "\u{2014}");
    let s = s.replace(" % ", " \u{2014} ").replace("% ", "\u{2014} ");
    regex_replace_kw(&s)
}

fn regex_replace_kw(s: &str) -> String {
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

const STYLE: &str = r#"<style>
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:Georgia,serif;font-size:14px;line-height:1.6;background:#faf8f2;color:#1e1a0e}
.wrap{display:flex;min-height:100vh}
.sidebar{width:200px;background:#f0ebe0;border-right:1px solid #d8d0c0;
  padding:1rem .75rem;flex-shrink:0;position:sticky;top:0;height:100vh;overflow-y:auto}
.sidebar h2{font-size:.75rem;font-weight:700;color:#7a5418;letter-spacing:.05em;margin-bottom:.6rem}
.alpha{display:flex;flex-wrap:wrap;gap:3px;margin-bottom:1rem}
.alpha a{display:inline-block;padding:2px 5px;font-size:.72rem;color:#7a5418;
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
.origin{color:#5a8040;font-style:italic}
.defs{margin:.2rem 0 .2rem 1rem}
.def{margin-bottom:.25rem;font-size:.85rem}
.grammar{color:#1a6860;font-size:.78rem}
.usage{color:#7a5418;font-weight:600;font-size:.82rem}
.tags{color:#8a8070;font-size:.75rem}
em.kw{color:#1a6860;font-style:italic}
.used-in{font-size:.72rem;color:#6a5c48;margin-top:.3rem}
.used-in a{color:#7a5418;text-decoration:none}
.used-in a:hover{text-decoration:underline}
@media(max-width:600px){
  .wrap{flex-direction:column}
  .sidebar{width:100%;height:auto;position:static}
  .content{padding:1rem}
}
</style>"#;

const SCRIPT: &str = r#"<script>
function doSearch(q){
  q=q.toLowerCase().trim();
  document.querySelectorAll('.entry').forEach(function(el){
    el.style.display=(!q||el.dataset.name.startsWith(q))?'':'none';
  });
  document.querySelectorAll('.letter-section').forEach(function(sec){
    var v=[...sec.querySelectorAll('.entry')].some(e=>e.style.display!=='none');
    sec.style.display=v?'':'none';
  });
}
document.getElementById('lod-search').addEventListener('input',function(){doSearch(this.value);});
</script>"#;

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
}

pub fn generate_html(conn: &Connection, event_name: Option<&str>) -> rusqlite::Result<String> {
    let words_sql = "SELECT w.id, w.name, t.name, w.source, w.year, w.rank, w.match_,
                            w.origin, w.origin_x
                     FROM words w
                     LEFT JOIN types t ON t.id=w.type_id
                     LEFT JOIN events es ON es.id=w.event_start_id
                     WHERE (?1 IS NULL OR es.name=?1)
                     ORDER BY LOWER(w.name)";

    let mut stmt = conn.prepare(words_sql)?;
    let rows: Vec<WordRow> = stmt
        .query_map(params![event_name], |r| {
            Ok(WordRow {
                id: r.get(0)?,
                name: r.get(1)?,
                type_name: r.get(2)?,
                source: r.get(3)?,
                year: r.get(4)?,
                rank: r.get(5)?,
                match_: r.get(6)?,
                origin: r.get(7)?,
                origin_x: r.get(8)?,
            })
        })?
        .filter_map(|r: rusqlite::Result<WordRow>| r.ok())
        .collect();

    let letters: Vec<char> = rows
        .iter()
        .filter_map(|w| w.name.chars().next().map(|c: char| c.to_ascii_uppercase()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    let mut html = String::with_capacity(rows.len() * 300);
    let title = match event_name {
        Some(ev) => format!("LOD — {}", esc(ev)),
        None => "Loglan Online Dictionary".to_string(),
    };
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n");
    html.push_str(&format!("<title>{}</title>\n", title));
    html.push_str(STYLE);
    html.push_str("</head>\n<body>\n<div class=\"wrap\">\n<nav class=\"sidebar\">\n");
    html.push_str("<h2>LOD</h2>\n");
    html.push_str(
        "<input id=\"lod-search\" class=\"search-box\" type=\"text\" placeholder=\"Search…\">\n",
    );
    html.push_str("<div class=\"alpha\">\n");
    for c in &letters {
        html.push_str(&format!("<a href=\"#L{}\">{}</a>\n", c, c));
    }
    html.push_str("</div>\n</nav>\n<main class=\"content\">\n");

    let mut def_stmt = conn.prepare(
        "SELECT grammar, usage, body, tags FROM definitions WHERE word_id=?1 ORDER BY position",
    )?;
    let mut afx_stmt =
        conn.prepare("SELECT affix FROM word_affixes WHERE word_id=?1 ORDER BY id")?;
    // EXISTS subquery avoids binding ?1 twice (rusqlite requires unique param count)
    let mut used_stmt = conn.prepare(
        "SELECT DISTINCT w2.name FROM words w2
         WHERE w2.id != ?1
           AND EXISTS (
             SELECT 1 FROM word_affixes wa
             WHERE wa.word_id = ?1
               AND LOWER(w2.name) LIKE '%'||LOWER(wa.affix)||'%'
           )
         ORDER BY w2.name LIMIT 60",
    )?;

    let mut cur_letter = '\0';
    for w in &rows {
        let first = w
            .name
            .chars()
            .next()
            .map(|c: char| c.to_ascii_uppercase())
            .unwrap_or('?');
        if first != cur_letter {
            if cur_letter != '\0' {
                html.push_str("</div></div>\n");
            }
            cur_letter = first;
            html.push_str(&format!(
                "<div class=\"letter-section\" id=\"L{0}\">\n<div class=\"letter-head\">{0}</div>\n<div>\n", first));
        }

        let affixes: Vec<String> = afx_stmt
            .query_map(params![w.id], |r| r.get(0))?
            .filter_map(|r: rusqlite::Result<String>| r.ok())
            .collect();

        let used_in: Vec<String> = used_stmt
            .query_map(params![w.id], |r| r.get(0))?
            .filter_map(|r: rusqlite::Result<String>| r.ok())
            .collect();

        html.push_str(&format!(
            "<div class=\"entry\" data-name=\"{}\">\n",
            w.name.to_lowercase()
        ));
        html.push_str(&format!("<div class=\"entry-name\">{}", esc(&w.name)));
        for a in &affixes {
            html.push_str(&format!(
                " <span style=\"font-size:.7rem;color:#5a8040;border:1px solid #5a8040;border-radius:2px;padding:0 3px\">{}</span>",
                esc(a)));
        }
        html.push_str("</div>\n");

        // Meta
        let mut meta_parts: Vec<String> = Vec::new();
        if let Some(ref origin) = w.origin {
            let ox = w.origin_x.as_deref().unwrap_or("");
            let ox_part = if !ox.is_empty() {
                format!(" = {}", esc(ox))
            } else {
                String::new()
            };
            meta_parts.push(format!(
                "<span class=\"origin\">&lt;{}{}&gt;</span>",
                esc(origin),
                ox_part
            ));
        }
        if let Some(ref m) = w.match_ {
            meta_parts.push(esc(m));
        }
        if let Some(ref t) = w.type_name {
            meta_parts.push(esc(t));
        }
        if let Some(ref s) = w.source {
            meta_parts.push(esc(s));
        }
        if let Some(ref y) = w.year {
            meta_parts.push(esc(y));
        }
        if let Some(ref r) = w.rank {
            meta_parts.push(esc(r));
        }
        if !meta_parts.is_empty() {
            html.push_str(&format!(
                "<div class=\"entry-meta\">{}</div>\n",
                meta_parts.join(" · ")
            ));
        }

        // Definitions
        type DefRow = (Option<String>, Option<String>, String, Option<String>);
        let defs: Vec<DefRow> = def_stmt
            .query_map(params![w.id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })?
            .filter_map(|r: rusqlite::Result<DefRow>| r.ok())
            .collect();

        if !defs.is_empty() {
            html.push_str("<div class=\"defs\">\n");
            for (grammar, usage, body, tags) in &defs {
                html.push_str("<div class=\"def\">");
                if let Some(u) = usage {
                    let u2 = u.replace('%', &esc(&w.name));
                    html.push_str(&format!("<span class=\"usage\">{} </span>", esc(&u2)));
                }
                if let Some(g) = grammar {
                    html.push_str(&format!("<span class=\"grammar\">({})</span> ", esc(g)));
                }
                html.push_str(&fmt_body(body));
                if let Some(t) = tags {
                    html.push_str(&format!(" <span class=\"tags\">[{}]</span>", esc(t)));
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
                html.push_str(&format!(
                    "<a href=\"#\" onclick=\"document.getElementById('lod-search').value='{eu}';doSearch('{eu}');return false\">{eu}</a>"));
            }
            html.push_str("</div>\n");
        }

        html.push_str("</div>\n"); // .entry
    }

    if cur_letter != '\0' {
        html.push_str("</div></div>\n");
    }
    html.push_str("</main>\n</div>\n");
    html.push_str(SCRIPT);
    html.push_str("\n</body>\n</html>\n");
    Ok(html)
}

/// Write HTML to a file on disk (bypasses FS plugin permissions)
pub fn write_html_to_file(
    conn: &Connection,
    path: &str,
    event_name: Option<&str>,
) -> rusqlite::Result<()> {
    let html = generate_html(conn, event_name)?;
    std::fs::write(path, html.as_bytes())
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
}
