//! Demo ledge plugin: a **live, memoized** `signups-stats` renderer.
//!
//! This is the counterpoint to the SDK's `data-table` example, ported onto the
//! same [`Block`] + typed-[`Sheet`] + [`cache::memoize`] surface. It reads the
//! author **sheet data source** at `source` (default `/data/signups`) via
//! `host::read_sheet` and renders aggregate stats over the signup rows: the
//! total count, the latest entry, and per-day counts.
//!
//! **Memoization is on.** A plugin's private KV cache is cleared only on a
//! *code* change, never a *data* change — a naive `cache_get`/`cache_put` keyed
//! on just the page path would keep serving the count from before the newest
//! signup forever. [`cache::memoize`] closes that gap by folding the sheet's
//! `host::source_version` into the cache key: a submission bumps the sheet's
//! version, which changes the key, which misses the cache and recomputes. The
//! empty state (an absent or empty sheet) is memoized too — it is correct until
//! the sheet appears or gains rows, and an absent source versions as `0`, so
//! creating the sheet changes the key just like editing it does.
//!
//! An earlier version of this plugin hand-wrote its own `single_fields` /
//! `scalar` / `html_escape` / `Sheet` decoding and ran **uncached** specifically
//! to dodge that stale-count bug (a path-only KV key can't tell a stale sheet
//! from a fresh one). None of that workaround remains: the typed host + a
//! version-keyed cache key make correct memoization the easy path instead of a
//! trap.
//!
//! The output is deterministic (it derives only from the sheet rows, no time or
//! randomness) and carries zero `data-aue-*` attributes — the host wraps the
//! block; the plugin emits only the inner HTML.
//!
//! Build + install:
//!
//! ```text
//! ledge plugin build --manifest-path plugin-src/signups-stats/Cargo.toml --project .
//! # or, from the justfile: `just install-plugins`
//! ```

use ledge_plugin_sdk::eds_html::escape;
use ledge_plugin_sdk::host;
use ledge_plugin_sdk::prelude::*;
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
            fields
                .str("source")
                .map(str::to_owned)
                .unwrap_or_else(|| Self::DEFAULT.to_owned()),
        )
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

/// One signup row, read from a sheet [`Row`]. Only the columns the stats view
/// needs are pulled out; a row missing either column still counts toward the
/// total, just with an empty display value for the missing one.
struct Signup {
    submitted_at: String,
    name: String,
}

impl Signup {
    fn from_row(row: &Row) -> Signup {
        Signup {
            submitted_at: row.get("submitted_at").unwrap_or_default().to_owned(),
            name: row.get("name").unwrap_or_default().to_owned(),
        }
    }
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

/// Render the `signups-stats` block: read the sheet, memoize on a per-page,
/// per-sheet-version key, and render live stats.
#[ledge::renderer("signups-stats")]
fn signups_stats(block: Block) -> Result<Html, RenderError> {
    let source = SheetPath::parse(block.fields());
    let dep = SourceDep::Sheet(source.as_str().trim_start_matches('/').to_owned());

    // Memoizing is safe now: the key carries the sheet's source-version, so a new
    // signup changes the key and misses. (This block previously ran uncached
    // BECAUSE a path-only KV key served stale counts after every submission.)
    cache::memoize(&format!("signups-stats:{}", block.page_path()), &[dep], || {
        Ok(render_stats(&Stats::compute(read_signups(&source))))
    })
}

/// Read + parse the signup rows for `source`. An absent sheet reads as no rows,
/// so the render degrades to the empty state rather than erroring.
fn read_signups(source: &SheetPath) -> Vec<Signup> {
    host::read_sheet(source.as_str())
        .map(|sheet| sheet.rows().iter().map(Signup::from_row).collect())
        .unwrap_or_default()
}

/// Render the stats panel: a prominent total, a latest line, and a per-day list.
/// A zero total (an absent or empty sheet) renders the empty state instead.
fn render_stats(stats: &Stats) -> Html {
    if stats.total == 0 {
        return empty_state();
    }

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
                escape(who),
                escape(day),
            )),
            None => html.push_str(&format!(
                "<p class=\"signups-stats-latest\">Latest: {}</p>",
                escape(who),
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
                escape(day),
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

ledge_plugin_sdk::plugin!();

#[cfg(test)]
mod tests {
    use super::*;
    use ledge_plugin_sdk::mock::MockHost;

    const TWO_SIGNUPS: &str = r#"{"total":2,"offset":0,"limit":2,":type":"sheet",
        "data":[{"submitted_at":"2026-01-01T10:00:00Z","name":"Ada Lovelace"},
                {"submitted_at":"2026-01-02T09:15:00Z","name":"Grace Hopper"}]}"#;
    const ONE_SIGNUP: &str = r#"{"total":1,"offset":0,"limit":1,":type":"sheet",
        "data":[{"submitted_at":"2026-01-01T10:00:00Z","name":"Ada Lovelace"}]}"#;

    /// A `signups-stats` block authored with its default `source` (every test
    /// here exercises the default; the field only needs to exist at all).
    fn signups_block() -> Block {
        Block::single("signups-stats", Fields::new())
    }

    #[test]
    fn stats_cover_two_signups_on_different_days() {
        let dep = SourceDep::Sheet("data/signups".into());
        MockHost::new()
            .sheet("/data/signups", TWO_SIGNUPS)
            .source_version(&dep, 1)
            .install(|| {
                let html = signups_stats(signups_block()).unwrap();
                assert!(html.contains("<span class=\"signups-stats-number\">2</span>"));
                assert!(html.contains("Latest: Grace Hopper on 2026-01-02"));
                // Both days show up in the per-day breakdown.
                assert!(html.contains("2026-01-01"));
                assert!(html.contains("2026-01-02"));
            });
    }

    #[test]
    fn an_absent_sheet_renders_the_empty_state_not_an_error() {
        MockHost::new().install(|| {
            let html = signups_stats(signups_block()).unwrap();
            assert!(html.contains("No signups yet."));
        });
    }

    #[test]
    fn a_bumped_source_version_recomputes_instead_of_serving_a_stale_count() {
        // THE regression this plugin exists to prove closed: a submission bumps
        // the sheet's source-version, so the SAME memoize key would otherwise
        // return the OLD count. One `MockHost` throughout, so the KV that took
        // the first render's result is the KV the second render consults.
        let dep = SourceDep::Sheet("data/signups".into());
        let mock = MockHost::new()
            .sheet("/data/signups", ONE_SIGNUP)
            .source_version(&dep, 1);
        let first = mock.install(|| signups_stats(signups_block()).unwrap());
        assert!(first.contains("<span class=\"signups-stats-number\">1</span>"));

        let mock = mock
            .sheet("/data/signups", TWO_SIGNUPS)
            .source_version(&dep, 2);
        let second = mock.install(|| signups_stats(signups_block()).unwrap());
        assert!(second.contains("<span class=\"signups-stats-number\">2</span>"));
        assert_ne!(
            first, second,
            "a bumped source version must recompute, not serve the cached count"
        );
    }
}
