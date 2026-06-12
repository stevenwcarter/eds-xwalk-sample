// Example ledge plugin panel bundle: a bespoke inspector for the `gallery` block.
//
// This is the in-browser authoring UI the editor loads when a block is claimed by
// a `#[ledge::panel("gallery")]` plugin. The server advertises this file's URL in
// `/__ledge/config` (`plugins.panels.gallery = "/plugins/gallery-panel/panel/index.js"`),
// and the editor `import()`s it and calls `mount(node, bridge)` when a `gallery`
// block is selected.
//
// ── Why this is hand-written and not the raw `jco transpile` output ──────────────
// `jco transpile` of a ledge plugin's wasm component produces an ESM that exports
// the component's WIT world — here `manifest()` and `render(input)` — NOT the
// editor's panel surface (`mount`/`unmount`). The panel bridge is a different,
// browser-side contract, so the panel ENTRY is authored by hand against it. The
// jco output is kept under `panel/jco-component/` for reference: a panel may
// `import('./jco-component/index.js')` and call its `render()` to show a live
// preview using the very same Rust renderer that runs on the server. This example
// keeps the preview self-contained (no wasm load) for clarity, but the
// jco-component is wired up and available.
//
// ── Panel bridge contract (see crates/ledge-editor/tests/fixtures/panel-mock) ────
//   export function mount(node, bridge): void   // render UI into `node`
//   export function unmount(node): void         // tear down
//   bridge = {
//     getProps(): object                  // the Single block's current fields,
//                                          //   serde-JSON shape (untagged FieldValue):
//                                          //   { image:{src,alt}, caption:"…", layout:"…" }
//     setProps(patch: object): void        // shallow-merge patch → block fields
//     subscribe(cb: (props)=>void): ()=>void   // observe external prop changes
//     host: { readPage(path), readMediaMeta(src),
//             selectMedia(): Promise<{src}|null> }   // async host reads + picker
//   }
//
// The `gallery` block is authored as Single content, so its fields are a flat map.
// This panel edits three of them: `image` (a Media value {src,alt}), `caption`
// (Scalar text), and `layout` (Scalar, one of "grid" | "row"). The "Choose…"
// button beside Image src opens the editor's media picker via
// `host.selectMedia()` and adopts the chosen path into `src` (filling `alt` from
// the filename only when it is empty).

const LAYOUTS = ["grid", "row"];

function str(v) {
  return typeof v === "string" ? v : "";
}

// A Media FieldValue serializes as { src, alt, width?, height? }; read its parts
// defensively (the field may be absent or another shape mid-authoring).
function mediaPart(props, key) {
  const m = props && props.image;
  return m && typeof m === "object" ? str(m[key]) : "";
}

// Derive a friendly default alt from an asset path: basename, drop extension,
// turn separators into spaces, collapse whitespace. Used only to fill an EMPTY
// alt (the media store has no stored alt text).
function humanizeAlt(path) {
  const base = String(path).split("/").pop() || "";
  const noExt = base.replace(/\.[a-z0-9]+$/i, "");
  return noExt.replace(/[-_]+/g, " ").replace(/\s+/g, " ").trim();
}

const STYLE_ID = "ledge-gallery-panel-style";

// Inject the panel's own scoped CSS once. Plugin bundles must not depend on the
// host editor's private Tailwind utility classes, so the panel ships its own
// styles, scoped under `.gallery-panel` so nothing leaks into the shadow tree.
// The style is keyed by a single shared id; safe because the editor mounts at most
// one panel at a time (a panel owns the whole inspector for the selected block).
function ensureStyles(node) {
  const root = node.getRootNode ? node.getRootNode() : document;
  const host = root && root.getElementById ? root : document;
  if (host.getElementById && host.getElementById(STYLE_ID)) return;
  const style = document.createElement("style");
  style.id = STYLE_ID;
  style.textContent = `
    .gallery-panel { display: flex; flex-direction: column; gap: 0.75rem; padding: 0.25rem 0; }
    .gallery-panel-field { display: flex; flex-direction: column; gap: 0.25rem;
      font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.04em; }
    .gallery-panel-field input,
    .gallery-panel-field select { width: 100%; box-sizing: border-box;
      padding: 0.375rem 0.5rem; font-size: 0.875rem; text-transform: none; letter-spacing: normal; }
    .gallery-panel-src-row { display: flex; gap: 0.375rem; align-items: stretch; }
    .gallery-panel-src-row input { flex: 1 1 auto; }
    .gallery-panel-choose { flex: 0 0 auto; padding: 0.375rem 0.625rem;
      font-size: 0.75rem; cursor: pointer; white-space: nowrap; }
  `;
  // Append to the shadow root if available (so it is scoped to the editor tree),
  // else to <head> as a fallback.
  if (root && root.appendChild && root !== document) {
    root.appendChild(style);
  } else {
    document.head.appendChild(style);
  }
}

export function mount(node, bridge) {
  ensureStyles(node);
  const props = bridge.getProps();

  const root = document.createElement("div");
  root.className = "gallery-panel";

  // ── Image src (with a Choose… button that opens the editor media picker) ──
  // A composite field (label text + input + button), so it is a <div> rather than
  // a <label>: a <label> may own only one labelable control, and the button is a
  // second one — keep them siblings under a plain container instead.
  const srcField = document.createElement("div");
  srcField.className = "gallery-panel-field";
  const srcLabel = document.createElement("span");
  srcLabel.textContent = "Image src";
  const srcRow = document.createElement("div");
  srcRow.className = "gallery-panel-src-row";
  const src = document.createElement("input");
  src.type = "text";
  src.className = "gallery-panel-src";
  src.value = mediaPart(props, "src");
  const choose = document.createElement("button");
  choose.type = "button";
  choose.className = "gallery-panel-choose";
  choose.textContent = "Choose…";
  srcRow.append(src, choose);
  srcField.append(srcLabel, srcRow);

  // ── Image alt ────────────────────────────────────────────────────────────
  const altField = document.createElement("label");
  altField.className = "gallery-panel-field";
  altField.textContent = "Image alt";
  const alt = document.createElement("input");
  alt.type = "text";
  alt.className = "gallery-panel-alt";
  alt.value = mediaPart(props, "alt");
  altField.appendChild(alt);

  // ── Caption ──────────────────────────────────────────────────────────────
  const capField = document.createElement("label");
  capField.className = "gallery-panel-field";
  capField.textContent = "Caption";
  const caption = document.createElement("input");
  caption.type = "text";
  caption.className = "gallery-panel-caption";
  caption.value = str(props && props.caption);
  capField.appendChild(caption);

  // ── Layout (a closed set → a <select>, not a free text field) ─────────────
  const layoutField = document.createElement("label");
  layoutField.className = "gallery-panel-field";
  layoutField.textContent = "Layout";
  const layout = document.createElement("select");
  layout.className = "gallery-panel-layout";
  for (const opt of LAYOUTS) {
    const o = document.createElement("option");
    o.value = opt;
    o.textContent = opt;
    layout.appendChild(o);
  }
  layout.value = LAYOUTS.includes(str(props && props.layout))
    ? str(props.layout)
    : LAYOUTS[0];
  layoutField.appendChild(layout);

  // Writing the `image` Media value back requires sending the whole object, since
  // setProps shallow-merges at the field level (not within the Media value).
  const pushImage = () => {
    bridge.setProps({ image: { src: src.value, alt: alt.value } });
  };
  src.addEventListener("input", pushImage);
  alt.addEventListener("input", pushImage);
  // Open the editor's media picker and adopt the chosen image. Fill `alt` only
  // when it is empty, so a picked image never clobbers an alt the author typed.
  choose.addEventListener("click", async () => {
    const picked = await bridge.host.selectMedia();
    if (!picked || typeof picked.src !== "string") return; // cancelled / unavailable
    src.value = picked.src;
    if (!alt.value) alt.value = humanizeAlt(picked.src);
    pushImage();
  });
  caption.addEventListener("input", () => {
    bridge.setProps({ caption: caption.value });
  });
  layout.addEventListener("change", () => {
    bridge.setProps({ layout: layout.value });
  });

  // Reflect external edits (e.g. another panel, an undo) back into the inputs.
  // Guard against clobbering the field the author is mid-edit by only writing a
  // control when its value actually drifted.
  const unsubscribe = bridge.subscribe((next) => {
    const nextSrc = mediaPart(next, "src");
    const nextAlt = mediaPart(next, "alt");
    const nextCap = str(next && next.caption);
    const nextLayout = str(next && next.layout);
    if (src.value !== nextSrc) src.value = nextSrc;
    if (alt.value !== nextAlt) alt.value = nextAlt;
    if (caption.value !== nextCap) caption.value = nextCap;
    if (nextLayout && layout.value !== nextLayout) layout.value = nextLayout;
  });
  // Stash the unsubscribe on the node so unmount can find it without module state
  // (the editor may mount/unmount several panels over a session).
  node.__galleryPanelUnsubscribe = unsubscribe;

  root.append(srcField, altField, capField, layoutField);
  node.appendChild(root);
}

export function unmount(node) {
  if (typeof node.__galleryPanelUnsubscribe === "function") {
    node.__galleryPanelUnsubscribe();
    delete node.__galleryPanelUnsubscribe;
  }
  const root = node.querySelector(".gallery-panel");
  if (root) {
    root.remove();
  }
  // Remove the injected scoped style so repeated mount/unmount cycles don't
  // accumulate duplicates (mount re-injects on the next mount).
  const treeRoot = node.getRootNode ? node.getRootNode() : document;
  const style =
    treeRoot && treeRoot.getElementById ? treeRoot.getElementById(STYLE_ID) : null;
  if (style) style.remove();
}
