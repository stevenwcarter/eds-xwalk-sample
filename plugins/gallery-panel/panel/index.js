// Example ledge plugin panel bundle: the in-browser inspector for the
// `gallery` block, claimed by this plugin's `#[ledge::panel("gallery")]`
// (see `../src/lib.rs`). The server advertises this file's URL in
// `/__ledge/config` (`plugins.panels.gallery = "/plugins/gallery-panel/panel/index.js"`),
// and the editor `import()`s it and calls `mount(node, bridge)` when a `gallery`
// block is selected.
//
// ── The declarative field list IS the panel ───────────────────────────────────
// `definePanel` (served by ledge itself at `/plugins/_sdk/panel-runtime.js`, so
// no npm/bundler/build step of the panel author's own) owns the whole bridge
// contract: seeding controls from `bridge.getProps()`, writing edits back
// through `bridge.setProps()` (writing the WHOLE `image` object on any src/alt
// change, since `setProps` shallow-merges at field level only — a partial
// `{ image: { src } }` patch would silently drop `alt`), reflecting external
// edits via `bridge.subscribe()` without clobbering an in-progress keystroke,
// the "Choose…" button wired to `bridge.host.selectMedia()`, and teardown on
// `unmount`. Compare this file's git history for what a hand-written panel
// used to cost (~180 lines of `createElement` plumbing plus a hand-injected
// stylesheet) — see `crates/ledge/src/plugin/panel_runtime.js` for the full
// field-type catalog and the bridge contract it implements.
//
// The `gallery` block is authored as Single content, so its fields are a flat
// map: `image` (a Media value `{src,alt}`), `caption` (Scalar text), and
// `layout` (Scalar, one of "grid" | "row").
//
// ── Why this is hand-written and not the raw `jco transpile` output ──────────
// `jco transpile` of a ledge plugin's wasm component produces an ESM that
// exports the component's WIT world — here `manifest()` and `render(input)` —
// NOT the editor's panel surface (`mount`/`unmount`). The panel bridge is a
// different, browser-side contract, so the panel ENTRY is authored by hand
// against it (`definePanel` below IS that hand-authoring — just six lines of
// it). The jco output is kept under `panel/jco-component/` for reference: a
// panel may `import('./jco-component/index.js')` and call its `render()` to
// show a live preview using the very same Rust renderer that runs on the
// server. See `crates/ledge-plugin-sdk/examples/README.md`.
import { definePanel } from '/plugins/_sdk/panel-runtime.js';

export const { mount, unmount } = definePanel({
  fields: [
    { name: 'image', type: 'media', label: 'Image' },
    { name: 'caption', type: 'text', label: 'Caption' },
    {
      name: 'layout', type: 'select', label: 'Layout', options: ['grid', 'row'],
    },
  ],
});
