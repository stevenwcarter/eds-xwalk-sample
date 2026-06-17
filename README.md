# Your Project's Title...
Your project's description...

## Environments
- Preview: https://main--eds-xwalk-sample--stevenwcarter.aem.page/
- Live: https://main--eds-xwalk-sample--stevenwcarter.aem.live/

## Documentation

Before using the aem-boilerplate, we recommend you to go through the documentation on [www.aem.live](https://www.aem.live/docs/), more specifically:
1. [AEM Authoring](https://www.aem.live/docs/aem-authoring)
2. [Universal Editor Tutorial](https://www.aem.live/developer/ue-tutorial)
3. [Component Model Definitions](https://www.aem.live/developer/component-model-definitions)
4. [Authoring Path Mapping](https://www.aem.live/developer/authoring-path-mapping)

## Prerequisites

- nodejs 20 or newer
- AEM Cloud Service release 2026.4 or newer

## Installation

```sh
npm i
```

## Linting

```sh
npm run lint
```

## Ledge demo: sheet-backed signups

This repo doubles as a demo for [Ledge](../ledge-cms) — a local-first,
self-hostable Rust content engine that serves EDS-compatible content. The
`/signups` page wires three blocks around one shared data sheet to show the full
authoring-to-render loop:

- a **`form`** block that writes submissions to the `/data/signups` sheet
  (public `POST /__ledge/forms/submit`, no editor or auth required);
- a **`signups-table`** block that renders the sheet as a table server-side
  (a minijinja `sheet()` global); and
- a **`signups-stats`** block — a **sandboxed WASM plugin** that reads the same
  sheet via the `read-sheet` host capability and renders a total, the latest
  signup, and a per-day breakdown.

Submitting the form appends a row to the sheet; because Ledge invalidates the
affected blocks on write, the table and stats blocks re-render with the new data
on the next request. Everything is rendered **server-side** (the server is the
only renderer), so the delivery port stays cache-friendly while reflecting live
content.

### Prerequisites

- A sibling **`../ledge-cms`** checkout (the Ledge engine — built from source).
- The **`wasm32-unknown-unknown`** Rust target: `rustup target add wasm32-unknown-unknown`.
- **`wasm-tools`** on `PATH` (componentizes the plugin): `cargo install wasm-tools`.
- **`just`** (the command runner): `cargo install just`.

### How to run

```sh
just dev
```

`just dev` builds + installs the `signups-stats` plugin, then runs the Ledge
server (from `../ledge-cms`) against this project. Then visit:

- http://localhost:3000/signups — the demo page (delivery port).

The authoring editor is on http://localhost:3001 and the published port on
http://localhost:3002 (pass `just dev <port> <authoring-port>` to override).

### The plugin

The plugin source lives in **`plugin-src/signups-stats/`** (a small Rust
`cdylib` built against the Ledge plugin SDK). It is compiled to a WASM component
and installed into the served `plugins/` tree by two timestamp-gated recipes:

```sh
just build-plugins     # compile + componentize (only if sources changed)
just install-plugins   # copy the built component into plugins/signups-stats/
```

`just dev` runs `install-plugins` for you. `just clean` removes the build
artifacts.

### The seeded sheet

The `/data/signups` sheet is **seeded once** at server startup from the
project-root **`sheets/data/signups.json`** (which ships one example row, Ada
Lovelace). The seed is **write-if-absent**: it never overwrites an existing
sheet, so form submissions **accumulate across restarts** — the content sheet
under `stores/preview/` is the source of truth once it exists.
