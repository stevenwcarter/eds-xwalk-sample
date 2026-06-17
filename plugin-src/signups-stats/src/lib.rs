//! Demo ledge plugin: a **live** `signups-stats` renderer.
//!
//! This is the counterpoint to the `data-table` example. It reads the author
//! **sheet data source** at `source` (default `/data/signups`) via the
//! `read-sheet` host capability and renders aggregate stats over the signup
//! rows: the total count, the latest entry, and per-day counts.
//!
//! Unlike `data-table`, this renderer **does NOT memoize** its HTML in the
//! plugin KV cache. Per-plugin KV is busted only on a *code* change, never on a
//! *sheet-data* change, so a memoizing stats block would serve stale numbers
//! after a new signup. Computing stats over a signups sheet is cheap, so it runs
//! fresh on every render; correct published-cache invalidation is handled by the
//! engine's sheet read-dependency, not by the plugin.
//!
//! The output is deterministic (it derives only from the sheet rows, no time or
//! randomness) and carries zero `data-aue-*` attributes — the host wraps the
//! block; the plugin emits only the inner HTML.
//!
//! Build + install:
//!
//! ```text
//! cargo build --manifest-path plugin-src/signups-stats/Cargo.toml \
//!   --target wasm32-unknown-unknown --release
//! wasm-tools component new \
//!   target/wasm32-unknown-unknown/release/signups_stats.wasm -o signups-stats.wasm
//! # drop into a project as plugins/signups-stats/signups-stats.wasm
//! ```

use ledge_plugin_sdk::bindings::ledge::plugin::host;
use ledge_plugin_sdk::prelude::*;
use ledge_plugin_sdk::serde_json;
use serde::Deserialize;
use std::collections::BTreeMap;

/// The sheet data source a `signups-stats` block reads. Wrapping the path in a
/// newtype names the *what* at every call site and keeps it from being confused
/// with the block's own `page_path`.
struct SheetPath(String);

impl SheetPath {
    /// Default sheet when the block omits a `source` field.
    const DEFAULT: &'static str = "/data/signups";

    /// Parse the block's `source` field into a sheet path, defaulting when absent.
    fn parse(fields: &Fields) -> Self {
        Self(
            scalar(fields, "source")
                .map(str::to_owned)
                .unwrap_or_else(|| Self::DEFAULT.to_owned()),
        )
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

/// The single-sheet EDS shape `host::read_sheet` returns: a `data` array of
/// string-keyed cell objects (every EDS cell is a string).
///
/// This reads only the **single-tab** shape. A multi-tab sheet derives to the
/// `:type:multi-sheet` envelope (no top-level `data`), so `data` defaults to
/// empty and the block renders its empty state rather than erroring.
#[derive(Deserialize)]
struct Sheet {
    #[serde(default)]
    data: Vec<BTreeMap<String, String>>,
}

/// One signup row, parsed from a sheet `data` entry. Only the columns the stats
/// view needs are pulled out; anything missing degrades gracefully (the row is
/// still counted, just with empty display fields).
struct Signup {
    submitted_at: String,
    name: String,
}

/// The aggregate stats computed over the signup rows.
struct Stats {
    total: usize,
    latest: Option<Signup>,
    /// Per-day counts keyed by the `YYYY-MM-DD` date prefix, ascending by key.
    per_day: BTreeMap<String, usize>,
}

impl Stats {
    /// Compute the stats from the parsed rows in a single pass. The latest entry
    /// is the row with the lexicographically greatest `submitted_at` (RFC 3339
    /// sorts chronologically); per-day counts group by the date prefix.
    fn compute(rows: Vec<Signup>) -> Self {
        let total = rows.len();
        let mut per_day: BTreeMap<String, usize> = BTreeMap::new();
        let mut latest: Option<Signup> = None;

        for row in rows {
            if let Some(day) = day_of(&row.submitted_at) {
                *per_day.entry(day.to_owned()).or_insert(0) += 1;
            }
            let is_newer = latest
                .as_ref()
                .is_none_or(|cur| row.submitted_at > cur.submitted_at);
            if is_newer {
                latest = Some(row);
            }
        }

        Self {
            total,
            latest,
            per_day,
        }
    }
}

/// Render the `signups-stats` block: read the sheet fresh and render live stats.
///
/// Deliberately no `cache_get`/`cache_put` — see the module docs.
#[ledge::renderer("signups-stats")]
fn signups_stats(input: BlockInput) -> Result<Html, RenderError> {
    let content = decode_block_content(&input.content)
        .map_err(|e| RenderError::InvalidInput(e.to_string()))?;
    let fields = single_fields(&content);
    let source = SheetPath::parse(fields);

    // Reading the sheet records it as a read-dependency, so the engine drops the
    // published cache for this page when the sheet changes.
    let Some(json) = host::read_sheet(source.as_str()) else {
        return Ok(empty_state());
    };

    let rows = parse_signups(&json)?;
    if rows.is_empty() {
        return Ok(empty_state());
    }

    Ok(render_stats(&Stats::compute(rows)))
}

/// Parse the derived EDS sheet JSON into signup rows. The sheet is author-edited
/// free text, so missing columns simply become empty display fields rather than
/// dropping the row from the total.
fn parse_signups(sheet_json: &str) -> Result<Vec<Signup>, RenderError> {
    let sheet: Sheet = serde_json::from_str(sheet_json)
        .map_err(|e| RenderError::Internal(format!("sheet is not valid JSON: {e}")))?;
    let rows = sheet
        .data
        .into_iter()
        .map(|mut row| Signup {
            submitted_at: row.remove("submitted_at").unwrap_or_default(),
            name: row.remove("name").unwrap_or_default(),
        })
        .collect();
    Ok(rows)
}

/// Render the stats panel: a prominent total, a latest line, and a per-day list.
fn render_stats(stats: &Stats) -> Html {
    let mut html = String::from("<div class=\"signups-stats\">");

    html.push_str(&format!(
        "<div class=\"signups-stats-total\"><span class=\"signups-stats-number\">{}</span>\
<span class=\"signups-stats-label\">total signups</span></div>",
        stats.total,
    ));

    if let Some(latest) = &stats.latest {
        let who = display_name(&latest.name);
        match day_of(&latest.submitted_at) {
            Some(day) => html.push_str(&format!(
                "<p class=\"signups-stats-latest\">Latest: {} on {}</p>",
                html_escape(who),
                html_escape(day),
            )),
            None => html.push_str(&format!(
                "<p class=\"signups-stats-latest\">Latest: {}</p>",
                html_escape(who),
            )),
        }
    }

    if !stats.per_day.is_empty() {
        html.push_str("<ul class=\"signups-stats-per-day\">");
        // Most recent day first (BTreeMap iterates ascending).
        for (day, count) in stats.per_day.iter().rev() {
            html.push_str(&format!(
                "<li><span class=\"signups-stats-day\">{}</span>\
<span class=\"signups-stats-count\">{}</span></li>",
                html_escape(day),
                count,
            ));
        }
        html.push_str("</ul>");
    }

    html.push_str("</div>");
    html
}

/// The graceful empty state shown when the sheet is missing or has no rows.
fn empty_state() -> Html {
    String::from("<div class=\"signups-stats signups-stats-empty\"><p>No signups yet.</p></div>")
}

/// The `YYYY-MM-DD` date prefix of an RFC 3339 timestamp, or `None` if the value
/// is too short to carry one.
fn day_of(submitted_at: &str) -> Option<&str> {
    submitted_at.get(..10)
}

/// A non-empty signup name, falling back to a generic label so the latest line
/// always reads sensibly even when the row omits a name.
fn display_name(name: &str) -> &str {
    if name.trim().is_empty() {
        "someone"
    } else {
        name
    }
}

/// The fields of a block, regardless of which `BlockContent` shape it uses. A
/// `signups-stats` block is authored as `Single`, but reading from
/// `Items`/`Container` too keeps the renderer robust.
fn single_fields(content: &BlockContent) -> &Fields {
    match content {
        BlockContent::Single { fields } | BlockContent::Container { fields, .. } => fields,
        BlockContent::Items { items } => items
            .first()
            .unwrap_or(EMPTY_FIELDS.get_or_init(Fields::new)),
    }
}

/// A shared empty `Fields` for the no-items fallback, so [`single_fields`] can
/// return a reference without allocating per call.
static EMPTY_FIELDS: std::sync::OnceLock<Fields> = std::sync::OnceLock::new();

/// Read a `Scalar` field as `&str`; `None` if absent or a non-scalar type.
fn scalar<'a>(fields: &'a Fields, key: &str) -> Option<&'a str> {
    match fields.get(key)? {
        FieldValue::Scalar(s) => Some(s),
        _ => None,
    }
}

/// Escape `&`, `<`, `>`, and `"` for safe embedding in HTML text and attributes.
fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}

ledge_plugin_sdk::plugin!();
