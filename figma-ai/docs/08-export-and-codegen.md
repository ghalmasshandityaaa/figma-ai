# 08 — Export & Code Generation

> **Status:** Normative. This document defines Kanvas export artifacts, the shared export IR, code emitters,
> the emitter plugin contract, and the determinism guarantees the rest of the product depends on.
> Canonical entity names come from [`02-data-model.md`](./02-data-model.md); the export UI (presets, job
> progress, history) is specified in [`04-feature-spec-systems.md`](./04-feature-spec-systems.md); agent
> tool invocation is specified in [`05-ai-agent-layer.md`](./05-ai-agent-layer.md); worker, queue, and
> storage infrastructure are specified in [`06-architecture.md`](./06-architecture.md).
>
> Every identifier introduced here that is not already in 02 is marked in §2.7 and must be registered in
> the glossary of [`README.md`](../README.md), per 02 §2.

---

## 1. Scope & Principles

Export is a **compiler**, not a screenshot button. It consumes an immutable document revision plus an explicit configuration and produces one of four artifact classes. Nothing in an artifact may depend on the exporting user, the wall clock, the client locale, or the machine that ran the job.

| Class | Targets | In scope | Out of scope |
|---|---|---|---|
| Assets | PNG, JPG, WebP, SVG, PDF, sprite sheets | Frames, selections, components, slices; theme-pinned rendering; multi-scale | Video, GIF, CMYK print profiles, 3D |
| Design data | `DesignJSON`, W3C Design Tokens, CSS vars, Tailwind theme, TS tokens, a11y report | Structure, layout, tokens, components, interactions, annotations | Editing round-trip of arbitrary third-party files |
| Code | React + Tailwind (primary), HTML + CSS, Tailwind HTML, React Native, Flutter | Presentational components, token plumbing, prototype navigation | Business logic, data fetching, auth, persistence, backend |
| Packages | ZIP + `manifest.json`; signed URL/clipboard/file delivery | Streaming packaging, artifact metadata, licenses | App store packaging, Xcode/Gradle project generation |

### 1.1 Fidelity tiers

Each target advertises exactly one primary tier. A target may exceed it locally, but may never silently fall below it; every fallback produces a `Diagnostic` and a manifest entry.

| Tier | Artifacts | Guarantee | Verified by | Explicitly not guaranteed |
|---|---|---|---|---|
| **Pixel-exact** | PNG, JPG, WebP | Same plan + renderer build produces identical bytes; geometry, clips, effects, and text match the editor renderer | Byte hash + visual diff against committed goldens | Cross-OS/GPU equality, print color, subpixel text identical across browser versions |
| **Vector-exact** | SVG, PDF | Paths, transforms, clips, solid/gradient paints, and embedded/subset text remain vector; no lossy resampling | XML/PDF structure snapshots + geometry probes | Unsupported filter/blend fallbacks, live text shaping in every viewer |
| **Structure-faithful** | React/Tailwind, HTML/CSS, RN, Flutter | DOM/widget tree, layout intent, token references, semantics, and interactions are faithful; code compiles | TS/Flutter compile, ESLint, Playwright, widget tests | Pixel equality after font substitution, runtime viewport reflow |
| **Token-faithful** | `DesignJSON`, DTCG tokens, CSS/Tailwind/TS token files | Names, types, values, aliases, modes, and theme selections survive export and re-import | JSON Schema + round-trip property tests | Plugin data not modeled by 02, component publish history |

### 1.2 Deterministic output

Determinism is defined at the artifact level:

```
bytes = E(planHash, emitter.name + "@" + emitter.version, canonicalOptions, toolchainPin)
```

`toolchainPin` covers the raster renderer build, font binaries, subsetter, formatter, minifier, and ZIP writer versions. The same tuple must produce **byte-identical** output. This is not cosmetic:

- **Golden tests** can compare bytes instead of fuzzy images, so regressions are attributable.
- **Agents** can cache, diff, and retry `kanvas.export_code` without producing spurious changes; the dry-run diff in §8.6 is only meaningful if unchanged input yields unchanged output.
- **Cache correctness** in §9.2 depends on `planHash` being a complete description of the output.
- **Supply chain** consumers can reproduce a build from the recorded revision and toolchain pin.

Normative rules:

1. JSON is canonical: keys sorted by UTF-16 code unit, two-space indent, LF endings, final newline; `undefined` omitted, empty arrays omitted, `-0` normalized to `0` (02 §17).
2. Numbers carry at most 3 decimals in design data and CSS, 4 in PDF coordinates. Trailing zeros are stripped; exponent notation is forbidden. Colors are lowercase `#rrggbb`/`#rrggbbaa`.
3. No wall-clock timestamps, hostnames, absolute paths, random values, UUIDs, or locale-dependent formatting appear by default. ZIP entry mtimes are fixed at `1980-01-01T00:00:00Z`; PDF dates derive from `FileMeta.updatedAt` or the explicit `KANVAS_EXPORT_EPOCH`.
4. Generated IDs are derived: `k-` + first 10 hex chars of `sha256(node.id)` + `-` + role. Collisions are resolved by appending `-2`, `-3` in paint order.
5. Ordering never depends on `Object.keys` insertion order, hash-map iteration, worker completion order, or filesystem enumeration. Children follow `children[]` (paint order, 02 I2); files sort by POSIX path.
6. Locale-sensitive operations are banned. Case conversion uses the pinned Unicode data version; text measurement uses the pinned `FontRegistry`.
7. Emitters are pure functions of `ExportPlan` and options. They may not query the database, read the live document, fetch the network, or inspect the current date.
8. Timestamps are opt-in (`embedTimestamp: true`) and, when enabled, are excluded from the determinism contract and marked `deterministic: false` in the export manifest.
9. Raster determinism is scoped to one renderer triple (engine, version, font set). Client Konva and server Chromium are different triples; artifacts record which one produced them. Cross-triple output is validated visually, never claimed byte-identical.
10. Every approximation (rasterized gradient, substituted font, dropped blend mode) emits a stable diagnostic code and is listed in `manifest.json`.

### 1.3 Limits and edge cases

- One `KanvasFile` per job. Linked library components are flattened from their resolved instances; export never mutates or republishes a library.
- 02 §18.1 caps a file at 50,000 nodes and a frame at 2,000 nodes. Export enforces both; the 10k-node raster budget in §9.1 is a multi-frame/over-limit stress envelope, not a supported frame size.
- A scope with zero visible exportable nodes fails with `E_SCOPE_EMPTY`; hidden nodes are excluded unless `includeHidden` is set, in which case they are emitted with `visibility="hidden"`/`display:none`.
- `SECTION` nodes are organizational and never render (02 §6); selecting one exports its descendants.
- Locked nodes are exported normally. Permissions affect who may start a job, not the pixels.
- Multi-file batches and server-side “export whole workspace” are out of scope for v1.

### 1.4 Acceptance criteria

1. Running the same job twice, with `NODE_OPTIONS` locales and `TZ` changed between runs, yields identical SHA-256 hashes for every artifact and for the ZIP.
2. Inserting keys into input JSON objects in a different order does not change `planHash` or output.
3. Every artifact lists `source.revision`, `planHash`, emitter name/version, and toolchain pin.
4. Every raster fallback in SVG/PDF/code carries a diagnostic and a `manifest.json` entry.
5. A test asserts that no output contains the job start time, hostname, or absolute workspace path when `embedTimestamp` is false.

---

## 2. Export Pipeline Architecture

### 2.1 Shared `ExportPlan` IR

All emitters consume one resolved, target-specific intermediate representation. The plan is built once, validated, hashed, and never mutated during emission.

```ts
interface ExportPlan {
  exportPlanVersion: 1;
  planHash: string;                       // sha256 of the canonical plan payload
  source: { fileId: FileId; revision: number; pageIds: PageId[]; rootNodeIds: NodeId[]; themeId: string; themeName: string };
  target: ExportTarget;
  options: ExportOptions;                 // defaults applied; unknown keys rejected
  frames: PlannedFrame[];                 // export roots with solved LayoutResult
  nodes: Record<NodeId, PlannedNode>;     // flattened, laid out, variables resolved
  tokens: ResolvedTokens;                 // aliases fully resolved for the selected theme
  components: PlannedComponent[];         // masters, variant axes, property definitions
  interactions: Interaction[];            // destinations remapped to scope
  assets: PlannedAsset[];                 // deduped by sha256
  fonts: PlannedFont[];                   // family/style, hash, license verdict, glyph set
  diagnostics: Diagnostic[];              // stable code + nodeId + message
}
```

`PlannedNode` is not a 02 `Node`. It carries: page-absolute `box` (x/y/width/height/rotation/scaleX/ scaleY), parent-relative `localBox`, resolved `fills`/`strokes`/`effects`, resolved `textStyle`, the post-override `characters`, applied truncation and line breaks, `isMask` role, and — for token-first codegen — the original `boundVariables` paths alongside their resolved values. `PlannedFrame` carries the 02 §7.1 `LayoutResult` plus `exportSettings`. `PlannedAsset` carries `hash`, dimensions, MIME type, storage key, license, and any transformed variant. `PlannedFont` carries family, style, `assetId` hash, subset glyph IDs, and the embedding verdict from §2.6.

The plan builder is the **only** component allowed to read the document store, resolve instances, or call the layout solver. Emitters that need data not present in the plan must fail with `E_PLAN_INCOMPLETE`; adding a field is a minor plan-version change, removing one is major.

### 2.2 Stages

| # | Stage | Input | Output | Purity / failure |
|---|---|---|---|---|
| 1 | Select scope | `pageIds`, `rootNodeIds`, `SliceNode` crops, selection | normalized scope, deduped roots, excluded `SECTION` | pure; `E_SCOPE_EMPTY`, `E_SCOPE_CROSS_FILE` |
| 2 | Resolve document view | snapshot at `revision`, component masters, `overrides` | flattened tree with overrides and variant selection applied | pure; dangling instance falls back to `detachedSnapshot` or `E_DANGLING_INSTANCE` |
| 3 | Layout solve | flattened tree, `FontRegistry`, `constraints`, `autoLayout` | 02 §7.1 `LayoutResult` per frame, page-absolute boxes | pure; `E_OVERFLOW_HARD` only in `strict` |
| 4 | Variable resolution | `Theme`, `VariableCollection` modes, `boundVariables` | `ResolvedTokens` + resolved node properties | pure, depth 32 (02 §9.1); cycle → `E_VARIABLE_CYCLE` |
| 5 | Asset extraction | `Asset` hashes, crop/filters, scale presets | optimized blobs, SVG sanitized, sprite atlas | I/O read-only; `E_ASSET_MISSING`, `E_ASSET_TOO_LARGE` |
| 6 | Emit | `ExportPlan` + emitter options | `EmittedFile[]` in memory or temp dir | pure; `E_EMITTER`, `E_UNSUPPORTED_NODE` |
| 7 | Post-process | files, pinned Prettier/SVGO/PNG optimizer | formatted/minified files | deterministic; formatting failure is `E_FORMAT` |
| 8 | Package | files + export manifest | deterministic ZIP | streamed; `E_PACKAGE_TOO_LARGE` |
| 9 | Deliver | artifact ref | signed URL, clipboard payload, or direct download | `E_STORAGE`, URL TTL 15 min |

Stages 1–5 are target-independent except for scale/format needs; their output is reused across targets within one job when the theme and scope match. Stage 5 never mutates source assets: optimization results are content-addressed and cached separately.

### 2.3 Client-side vs server-side execution

| Target | Client (browser) | Server export worker | Default | Fidelity note |
|---|---|---|---|---|
| PNG/JPG/WebP | Konva `stage.toBlob`, `pixelRatio = scale`; hard cap 2k nodes / 4096 px | Headless Chromium, pinned build, software raster, `deviceScaleFactor` | Server for jobs; client for quick single-frame export | Same renderer triple only for determinism; server required for tiling and `BACKGROUND_BLUR` |
| SVG | Pure TS serializer | Same serializer | Either | Byte-identical across runtimes |
| PDF | Not supported in v1 (font subsetting requires Node) | `pdf-lib` + `fontkit`, optional Chromium for effect raster fallback | Server | Vector text and paths; effects rasterized |
| DesignJSON / tokens / a11y | Pure TS | Same | Either | Byte-identical |
| React/HTML/RN/Flutter | Pure TS (preview + download) | Same emitters in worker | Either | Worker preferred for >500 nodes or ZIP |
| Sprite sheets | Client canvas up to 256 icons | Chromium/canvas, streaming | Server for jobs | Atlas packing is deterministic |
| ZIP | Streaming writer, cap 100 MB | Streaming writer, cap 2 GB | Server for jobs | Fixed metadata |

The worker hosts one warm Chromium process per node, recycled every 200 renders or 10 minutes to bound memory. Client rendering is never used to produce an artifact that is later certified as server-rendered; the manifest records `renderer`.

### 2.4 Job model

```ts
type ExportJobState = 'QUEUED' | 'RUNNING' | 'SUCCEEDED' | 'FAILED' | 'CANCELLED';

interface ExportJob {
  id: string; fileId: FileId; revision: number; requestedBy: string;   // exp_<uuidv7>; u_* | ag_*
  target: ExportTarget; scope: ExportScope; options: ExportOptions; state: ExportJobState;
  progress: { stage: ExportStage; completed: number; total: number; percent: number };
  attempts: number; maxAttempts: number; timeoutMs: number; idempotencyKey?: string;
  planHash?: string; error?: ExportError; artifacts: ExportArtifact[];
  createdAt: number; startedAt?: number; finishedAt?: number;
}

interface ExportArtifact {
  id: string; jobId: string; path: string; mimeType: string;   // art_<uuidv7>
  bytes: number; sha256: string; storageKey: string; expiresAt?: number;
}
```

Allowed transitions: `QUEUED → RUNNING`, `QUEUED → CANCELLED`, `RUNNING → SUCCEEDED | FAILED | CANCELLED`. A retry stays in `RUNNING` and increments `attempts`; it never returns to `QUEUED`. Timeout produces `FAILED` with `E_TIMEOUT`, not `CANCELLED`.

Progress events are emitted on channel `export:{jobId}` (Supabase Realtime) or SSE for direct clients: `STATE`, `PROGRESS`, `WARNING`, `ARTIFACT`, `ERROR`. `percent` is monotonic and weighted by stage: scope 5%, resolve 10%, layout 20%, variables 5%, assets 20%, emit 30%, package 10%. Events are throttled to 10 Hz; the final `SUCCEEDED` always carries `percent: 100`.

Cancellation is cooperative: `AbortSignal` is checked between nodes and between 4,096 px tiles; Chromium pages are closed, temp files deleted, and no artifact is published. p95 cancellation latency is 2 s. Retries occur only for `E_STORAGE`, worker crash, and Chromium navigation failure; validation, licensing, scope, and emitter errors are terminal. Backoff is 1 s, 4 s with jitter derived from `job.id`, not `Math.random()`.

Quotas (per workspace unless noted): 4 concurrent jobs, 2 per user; 60 jobs/hour/user; 500 jobs/day; 5,000 artifacts; 5 GiB stored; 500 megapixels rendered/day; 2 GiB per ZIP; 5,000 files per ZIP. Exceeding a quota returns `E_QUOTA` with `Retry-After`. Artifacts live under `exports/{fileId}/{jobId}/{target}/...`; ephemeral artifacts expire after 7 days, explicitly saved exports after 90 days. Signed URLs expire after 15 minutes and are scoped to one artifact.

Default timeouts: PNG/WebP 30 s, SVG 15 s, PDF 120 s, code/tokens/DesignJSON 30 s, sprite 60 s; hard cap 10 minutes. `idempotencyKey` returns the existing job for 24 hours when target, scope, and options match.

### 2.5 Fonts, embedding, and licensing

`FontRegistry` (02 §8) is the sole source of font bytes and metrics. A job fails closed when a font used by a pixel-exact or PDF target cannot be resolved, unless `fontFallbackPolicy: 'SUBSTITUTE'` is explicit.

| Font verdict | PNG/JPG/WebP | SVG | PDF | Code |
|---|---|---|---|---|
| Resolved, embedding allowed | Render with font | `textMode: AUTO` keeps live text; embed WOFF2/data URI only if self-contained | Embed deterministic subset | Reference family + local `@font-face` asset |
| Resolved, embedding restricted | Render (raster output) | Outline only if license allows outlines; else rasterize text region or fail | Rasterize text region or fail; never embed | Reference family; do not ship font binary; include fallback stack |
| Resolved, web restricted | Render | Outline/raster per above | Embed only if PDF embedding allowed | Reference family without `@font-face`; warn `W_FONT_WEB_RESTRICTED` |
| Missing | Fail, or substitute with `W_FONT_SUBSTITUTED` (not pixel-exact) | Fail, or outline unavailable → fail | Fail | Substitute with CSS stack + warning; fixed-height text may clip |

Licensing is data, not inference. `Asset.license` (02 §10) is copied into the export manifest. A sidecar `FontLicenseRecord` keyed by `assetId` records `embed`, `outline`, `web`, and `rasterize` permissions as `ALLOWED | RESTRICTED | UNKNOWN`; `UNKNOWN` is treated as `RESTRICTED` for embedding and outlining. Restricted fonts may still be rendered into a raster image (a depiction, not a distribution), with `W_FONT_RASTERIZED` and the license name in the manifest. No export ships a font binary whose license does not explicitly allow redistribution.

PDF subsetting uses `fontkit`, sorted glyph IDs, a deterministic subset tag derived from `sha256(fontHash + sortedGlyphIds)`, and a `ToUnicode` CMap. SVG outlining uses a fixed flattening tolerance of 0.05 px and no hinting instructions. Text-as-text in SVG embeds a WOFF2 data URI only when the license permits web embedding and `svgMode: SELF_CONTAINED`; otherwise it references the family and records `W_SVG_FONT_EXTERNAL`.

### 2.6 New export-layer identifiers

| Identifier | Meaning | Persistence |
|---|---|---|
| `ExportTarget` | Artifact/emitter target union (§3–§8) | ephemeral |
| `ExportSetting` | Per-node scale/format/suffix stored in 02 `exportSettings` | document |
| `ExportPlan`, `PlannedNode`, `PlannedFrame`, `PlannedAsset`, `PlannedFont`, `ResolvedTokens` | Shared export IR | ephemeral, hashed |
| `ExportJob`, `ExportJobState`, `ExportArtifact` | Job and artifact records | Postgres + Storage |
| `DesignJSON`, `DesignJsonNode` | Versioned interchange document (§5) | artifact |
| `A11yAnnotation` | Per-node semantic sidecar (§5.4) | sidecar table keyed by `nodeId` |
| `Emitter`, `EmitterContext`, `EmitterResult`, `EmittedFile` | Plugin contract (§9) | code |
| `FontLicenseRecord` | Font permission sidecar (§2.5) | sidecar table keyed by `assetId` |
| `exp_`, `art_` | ID prefixes for jobs and artifacts | Postgres |

`ExportSetting` fills the shape referenced but not defined by 02 §6:

```ts
interface ExportSetting {
  format: 'PNG' | 'JPG' | 'WEBP' | 'SVG' | 'PDF';
  suffix: string;                          // appended before extension, e.g. "-hero"
  constraint:
    | { type: 'SCALE'; value: number }     // 0.5..4
    | { type: 'WIDTH'; value: number }     // px, derives scale
    | { type: 'HEIGHT'; value: number };
}
```

### 2.7 Limits and edge cases

- A theme may reference modes from multiple collections; resolution follows 02 §9.1 exactly. Missing mode IDs fall back to `defaultModeId` with `W_THEME_MODE_FALLBACK`.
- A variable cycle (max depth 32) renders magenta in assets, matching the editor; codegen emits a literal fallback and `E_VARIABLE_CYCLE` in `strict` mode.
- Dangling instances use `detachedSnapshot` (02 §11.5); without a snapshot they emit a 1×1 transparent placeholder plus `E_DANGLING_INSTANCE`.
- Cross-file linked components are flattened read-only; overrides are applied before flattening.
- A frame with `clipsContent: false` may paint outside its bounds; the raster canvas expands to `absoluteRenderBounds` only when `clipToFrame: false`, capped at 32,768 px per side.
- Font fallback never silently changes fixed-height text without a warning.

### 2.8 Acceptance criteria

1. A job transitions only through the allowed state graph; an illegal transition is rejected by the job service and covered by a test.
2. Cancellation of a 10k-node raster job completes within 2 s p95 and leaves no artifact.
3. A transient storage error is retried at most twice; a validation error is never retried.
4. `planHash` is stable across key insertion order, client vs server, and worker count.
5. A restricted font causes PDF export to fail or rasterize with `W_FONT_RASTERIZED`; no font bytes are embedded.
6. A job exceeding a quota returns `E_QUOTA` with `Retry-After` and does not enqueue work.

---

## 3. Raster & Vector Asset Export

Asset export reads `exportSettings` from `FrameNode`/`SliceNode` when present; an explicit job scope overrides presets. A `SLICE` renders the region it covers without painting itself (02 §6). Every artifact records its renderer, scale, theme, and source nodes.

### 3.1 Raster formats and scaling

| Format | Alpha | Encoder settings | Determinism pin | Notes |
|---|---|---|---|---|
| PNG | Yes | zlib level 9, adaptive filter, no ancillary chunks, no ICC unless `embedIcc` | Pinned `pngjs`/Chromium build | Default for UI assets and transparency |
| JPG | No | Quality 90, 4:2:0, baseline, no progressive, no EXIF | Pinned encoder | Flattened onto `background`; alpha request is `E_FORMAT_ALPHA` |
| WebP | Yes | Quality 90 lossy or `lossless: true`, method 4, no metadata | Pinned `libwebp` | Best for photographic fills and thumbnails |
| AVIF | Yes | Roadmap; not a v1 target | — | Deferred until decoder support is universal in target runtimes |

Rules:

- `scale` is 0.5–4.0. `constraint: WIDTH|HEIGHT` derives scale from the solved frame box, then clamps; clamping emits `W_CONSTRAINT_CLAMPED`. `SCALE` is applied to both axes.
- Device pixel ratio is a **preview** concern. Export dimensions are `max(1, round(layout.width * scale)) × max(1, round(layout.height * scale))`; client DPR is never multiplied in. This prevents Retina clients from producing different bytes.
- Pixel snapping happens once, after layout and at final scale. Box edges are rounded to device pixels; stroke weights are not snapped (that would change the design). Text is rendered by the pinned engine.
- `background` is `TRANSPARENT` (default for PNG/WebP), `FRAME` (paint the frame fill), `PAGE` (page background from 02 §13), or `CUSTOM` with an `RGBA`. JPG always composites onto an opaque color, default `#ffffff`; the manifest records it.
- Color is sRGB, 8-bit per channel. Document colors are interpreted as sRGB; no CMYK conversion. ICC profile embedding is off by default; when on, the pinned sRGB profile is embedded byte-identically.
- `IMAGE` paints apply `scaleMode` (`FILL`, `FIT`, `CROP`, `TILE`), `cropTransform`, `rotation`, and `filters` into a deterministic derived asset before rasterization. CSS filters are never used for raster output, so PNG/JPG/WebP match the editor's baked image result.
- Node `opacity < 1` is rendered as a group: node plus children are composited offscreen, then blended, matching Figma/CSS group semantics rather than per-child alpha multiplication.
- Effects are rasterized at target scale: `DROP_SHADOW` expands the render box by `|offset| + radius + spread`, `LAYER_BLUR` by `2 × radius`, `BACKGROUND_BLUR` samples the already painted backdrop inside the frame. Clipping is applied after effects only when `clipsContent`.
- Masks follow 02 `isMask`: within one parent, a mask node clips the siblings painted after it until the next mask or the end of `children[]`. Mask nodes themselves are not painted. Nested masks intersect. A mask never crosses a container boundary.
- `visible: false` nodes are omitted; `opacity: 0` nodes are omitted from raster output but retained in DesignJSON and code.
- Soft mask edges are antialiased at the target scale; no post-resize sharpening is applied.

### 3.2 Large frames and tiling

Frames whose scaled side exceeds 8,192 px are tiled. Tiles are 4,096 px, with a 64 px bleed on interior edges to carry shadows and blurs; the bleed is cropped from the delivered tile.

```json
// tiles.json (inside the ZIP; one per tiled artifact)
{ "version": 1, "pixelWidth": 12288, "pixelHeight": 8192, "tileWidth": 4096, "tileHeight": 4096,
  "bleed": 64, "columns": 3, "rows": 2,
  "tiles": [{ "path": "Login@2x-r0c0.png", "x": 0, "y": 0, "width": 4096, "height": 4096 }] }
```

Hard caps: 32,768 px per side, 268,435,456 pixels per artifact, 64 tiles. Exceeding a cap returns `E_TOO_LARGE` with the maximum allowed scale for the frame. Tiled artifacts are never auto-stitched on the client; the server can emit a stitched PNG only when the result is within the pixel cap and memory budget. Tiling is unavailable for SVG and PDF.

### 3.3 SVG mapping

SVG output is a semantic tree (not an embedded raster) with absolute geometry; auto layout is flattened because SVG has no flex model. Children remain in `children[]` paint order.

| 02 node | SVG output | Rules |
|---|---|---|
| `FRAME` | `<g>` + optional background `<rect>` | `clipsContent` → `<clipPath>` including corner radii; effects may expand the group |
| `GROUP` | `<g>` | No clip, no background; group `opacity`/`blendMode` on the `<g>` |
| `SECTION` | omitted | Exports descendants only |
| `RECTANGLE` | `<rect>` or `<path>` | Uniform `cornerRadius` → `rx`/`ry`; per-corner `Corners` or `cornerSmoothing > 0` → path |
| `ELLIPSE` | `<ellipse>` or `<path>` | `arcData` (pie/donut) → path with arc commands; full ellipse stays `<ellipse>` |
| `LINE` | `<line>` | Stroke-only; dash/cap/join from `Stroke` |
| `ARROW` | `<path>` + marker `<defs>` | Arrowheads from `arrowStart`/`arrowEnd`; markers share deterministic IDs |
| `POLYGON` | `<polygon>` | `pointCount` 3–12, computed from the node box, rotation preserved |
| `STAR` | `<polygon>` | `pointCount` and `innerRadius`; no `<star>` element exists |
| `VECTOR` | `<path>` per `VectorPath` | `M/L/C/Q/Z` preserved; `fillRule` → `fill-rule`; `windingRule` recorded |
| `BOOLEAN` | single `<path>` | Boolean result precomputed; if the geometry engine fails, fall back to clip/mask approximation + `W_BOOLEAN_APPROXIMATED` |
| `TEXT` | `<text>` + `<tspan>` or `<path>` | See text rules below; live text by default when fonts are legal |
| `IMAGE` | `<image>` | Data URI by default (`svgMode: SELF_CONTAINED`); `LINK` writes relative asset paths |
| `INSTANCE` | `<g>` | Overrides applied, master structure flattened; no `<use>` across files |
| `COMPONENT` | `<g>` | Master renders like a frame; `COMPONENT_SET` is a `<g>` of its variants only if explicitly scoped |
| `SLICE` | omitted | Crop/export region only |
| mask node | `<mask>` or `<clipPath>` | `mask-type="alpha"` by default; simple shapes use `clipPath` for smaller output |

Paint and effect rules:

- `GRADIENT_LINEAR` and `GRADIENT_RADIAL` map to native `<linearGradient>`/`<radialGradient>` with `gradientUnits="userSpaceOnUse"`. The 2×3 `gradientTransform` is applied to the unit square, then to the node box; results are rounded to 4 decimals. `GRADIENT_ANGULAR` and `GRADIENT_DIAMOND` have no SVG equivalent and are rasterized into an `<image>` clipped to the shape (`W_GRADIENT_RASTERIZED`).
- `DROP_SHADOW` → `<feDropShadow>`; `INNER_SHADOW` → flood/offset/composite/blur chain; `LAYER_BLUR` → `<feGaussianBlur>`; `BACKGROUND_BLUR` → rasterized backdrop region (`W_BACKGROUND_BLUR_RASTERIZED`). Filter regions are explicit (`filterUnits="userSpaceOnUse"`) so output never clips a shadow. Gaussian `stdDeviation` is `radius / 2`; the `effects-zoo` golden is normative for calibration.
- Blend modes → CSS `mix-blend-mode` on the corresponding group/element. `PASS_THROUGH` emits no blend mode and must not create a new stacking context; if an ancestor forces isolation, emit `W_PASS_THROUGH_ISOLATED`.
- IDs are `k-<sha256(node.id)[0:10]>-<role>` (`clip`, `grad`, `filter`, `mask`, `marker`). IDs are unique per document and never derived from user-supplied names.
- Auto layout has no SVG representation: x/y are the solved positions. `layoutGrow`, `HUG`, and `FILL` are recorded as `data-kanvas-*` attributes only when `emitLayoutHints: true`.
- `textMode`: `TEXT` keeps `<text>` (requires legal, available font); `OUTLINE` converts glyphs to paths at 0.05 px tolerance; `AUTO` chooses outline when the font is missing, restricted, or uses complex shaping that the target viewers may not reproduce. Line boxes come from `TextDerived.baselineOffsets`; each line is a `<tspan x=... y=...>`. `textCase` is applied before shaping; `letterSpacing` maps to `letter-spacing`; rich `rangeStyles` become nested `<tspan>`s; `hyperlink` becomes `<a href>` after scheme validation.
- `textAutoResize: TRUNCATE` must be resolved before emission: the exporter truncates at grapheme boundaries, appends `…`, and records the applied `maxLines`. SVG has no portable line-clamp.
- Accessibility: root `<svg role="img">` with `<title>` from the frame name and `<desc>` from `metadata.description` when present. `a11y: 'FULL'` adds per-`IMAGE`/`VECTOR` titles and `aria-labelledby`; decorative nodes get `aria-hidden="true"`. Hidden nodes are absent, not `display:none`.
- `svgMode: CLEAN` emits two-space indentation, one attribute per line, stable attribute order. `MINIFIED` runs pinned SVGO with a fixed plugin list; `cleanupIds` is configured with the `k-` prefix, and plugins that reorder or merge geometry are disabled. Both modes must be semantically identical under the SVG structural test in §11.4.

### 3.4 PDF

PDF uses 1 px = 0.75 pt (96 dpi CSS reference). Each frame becomes a page by default; `pageMode: 'CONTACT_SHEET'` lays frames on a chosen page size (`A4`, `LETTER`, `CUSTOM`) with uniform scale-to-fit, never upscaling above 1.0 unless `allowUpscale`. Page bounds are capped at 14,400 pt per side and 500 pages per document; overflow returns `E_PDF_PAGE_LIMIT`.

- Paths, clips, solid and gradient paints, images, and text remain vector. Blend modes use PDF 1.7 transparency groups. `PASS_THROUGH` containers are flattened in paint order.
- Drop shadows, inner shadows, layer blurs, and background blurs are rasterized locally at 4× and embedded as image XObjects (`W_PDF_EFFECT_RASTERIZED`). `pdfEffectMode: 'VECTOR'` is roadmap and must not be advertised until shadow soft-mask goldens pass.
- Fonts are subset per family/style with `ToUnicode` maps so text is selectable and extractable. Missing glyphs render `.notdef` and fail the job (`E_FONT_GLYPH_MISSING`) unless substitution is allowed.
- Internal links: `Interaction` with `NAVIGATE` becomes a `GoTo` annotation only when the destination frame is in the same PDF; otherwise `W_PDF_LINK_DROPPED`. `OPEN_LINK` becomes a URI annotation only for `https:`, `http:`, `mailto:`, and `tel:` schemes.
- Metadata: `Title` = file/frame name, `Creator` = `Kanvas`, `Producer` = `Kanvas Export <version>`, `CreationDate`/`ModDate` from `FileMeta.updatedAt` or `KANVAS_EXPORT_EPOCH`. No author email or user ID is embedded by default. Page labels and bookmarks follow frame order.
- Reading order follows the node tree; tagged PDF/PDF-UA is roadmap, not a v1 claim.

### 3.5 Sprite, icon, preset, and packaging

Presets compile to `ExportSetting[]`; explicit settings win. `@1x/@2x/@3x` is the generic default.

| Preset | Scales | Formats | Naming / notes |
|---|---|---|---|
| `web` | 1, 2 | SVG + PNG | SVG primary, PNG fallback; `{layer}.{ext}` / `{layer}@2x.png` |
| `ios` | 1, 2, 3 | PDF (vector) + PNG | PDF for vector icons; `Contents.json` asset catalogs are roadmap |
| `android` | 1, 1.5, 2, 3, 4 | PNG + SVG (API 21+) | `ic_` prefix, density suffixes `mdpi`…`xxxhdpi`; adaptive icons roadmap |
| `flutter` | 1, 2, 3 | PNG + SVG | `assets/icons/{name}@{scale}x.png`; `pubspec.yaml` snippet in manifest |
| `sprite` | 1, 2, 3 | PNG + JSON + SVG symbols | Shelf-packed; `sprite.json` frame table; optional `sprite.svg` |

Naming tokens: `{page}`, `{frame}`, `{layer}`, `{component}`, `{theme}`, `{scale}`, `{suffix}`, `{format}`. Default `{layer}@{scale}x.{format}`; `{scale}` renders `1x`, `2x`, `1.5x`. Names are normalized to NFC, path separators and `<>:"\|?*` become `-`, control characters and leading/trailing dots/spaces are removed, UTF-8 length is capped at 120 bytes, and collisions get `-2`, `-3` in node order. Empty names fall back to the node type lowercased plus the 10-hex node hash.

Sprite packing is deterministic: sort icons by height descending, width descending, then `NodeId` ascending; shelf-pack with 2 px padding at 1× (scaled per density); reject atlases above 4,096 px per side. Empty sheets are not emitted. SVG symbol IDs use the same `k-` namespacing as §3.3.

ZIP layout for a multi-target job:

```
export-<jobId>/
├── manifest.json
├── png/Login@2x.png
├── svg/Login.svg
├── pdf/Login.pdf
├── code/                      # only for code targets
├── tokens/                    # only for token targets
├── assets/                    # deduped image/font binaries referenced by output
└── licenses/
    ├── fonts.json
    └── assets.json
```

`manifest.json` is the **export** manifest, distinct from the `.kanvas` manifest in 02 §17. It contains `manifestVersion`, `source` (fileId, revision, pageIds, themeId), `planHash`, `target`, `emitter` (name/version), `renderer`, `createdAt` (source timestamp), `files[]` (`path`, `sha256`, `bytes`, `mime`, `width`, `height`, `scale`, `nodeId`), `fonts[]` (family, style, license, embedded, subset), `assets[]` (sha256, license, attribution), `warnings[]`, and `deterministic`. Entries sort by path. The manifest is written last and is itself listed with its hash computed over the canonical payload.

### 3.6 Limits and edge cases

- Zero-area frames emit a 1×1 transparent PNG or an SVG with `width="0"`; code emits an empty component plus `W_EMPTY_FRAME`.
- Frames larger than the raster caps must reduce scale; PDF caps at 14,400 pt; SVG has no size cap but warns above 20,000 px.
- Tiling is unsupported for effects that require the full canvas (`BACKGROUND_BLUR` across tile boundaries); those jobs fall back to a single tile if within caps, otherwise fail `E_EFFECT_TILING`.
- `GRADIENT_ANGULAR`/`DIAMOND` always rasterize in SVG; PDF preserves them as native shading where its shading types allow, otherwise rasterizes locally.
- Boolean geometry failure is never silently flattened to a bounding box; it fails or uses the documented clip approximation with a warning.
- Source SVG assets are sanitized at import (02 §10) and again at export; a rejected asset fails `E_ASSET_UNSAFE` rather than being partially emitted.
- PDF text with unsupported shaping (e.g. missing GSUB coverage) fails subsetting rather than emitting wrong glyphs.

### 3.7 Acceptance criteria

1. PNG at 1×/2×/3× matches the committed goldens within the §11.3 visual threshold and has the exact expected pixel dimensions.
2. A 12,288 × 8,192 px frame tiles into six 4,096 px tiles with no visible seam on the `effects-zoo` fixture; `tiles.json` validates against its schema.
3. Every 02 node type maps to exactly one SVG construct in §3.3; unsupported combinations produce a diagnostic, not a silent omission.
4. SVG `CLEAN` and `MINIFIED` parse to equivalent geometry and attributes under the structural snapshot.
5. A PDF with 10 frames preserves selectable text, internal links, and vector paths; fonts are subsets with `ToUnicode`.
6. Sprite packing of the icon fixture is byte-identical across repeated runs and independent of input selection order.
7. `manifest.json` hashes match the delivered files and list every license and warning.

---

## 4. Design-Data Export

### 4.1 Canonical `DesignJSON`

`DesignJSON` is the versioned interchange artifact for structure-faithful consumers. It is emitted as `design.json` inside the export ZIP (assets referenced by relative path) or standalone with assets as data URIs when `assetsMode: 'EMBED'`. It is not the `.kanvas` format; it is a resolved projection intended for import, analysis, and downstream codegen.

```json
{
  "designJsonVersion": 1,
  "source": { "fileId": "f_0192", "revision": 4821, "pageIds": ["pg_login"], "themeId": "th_light", "planHash": "sha256:…" },
  "units": "px",
  "nodes": [{
    "id": "n_card", "type": "FRAME", "name": "Login Card", "parentId": "n_mobile", "children": ["n_title"],
    "box": { "x": 24, "y": 300, "width": 342, "height": 268, "rotation": 0, "scaleX": 1, "scaleY": 1 },
    "layout": { "mode": "AUTO", "direction": "VERTICAL", "wrap": "NO_WRAP", "itemSpacing": 16,
      "padding": { "top": 24, "right": 24, "bottom": 24, "left": 24 }, "primaryAxisAlign": "MIN", "counterAxisAlign": "MIN" },
    "style": { "fills": [], "strokes": [], "effects": [], "cornerRadius": 16, "opacity": 1, "blendMode": "NORMAL" },
    "tokenBindings": { "fills.0.color": "color/bg/surface/raised" }, "text": null, "component": null }],
  "tokens": { "collections": [], "variables": [], "themes": [], "styles": [] },
  "components": [], "interactions": [], "assets": [], "diagnostics": []
}
```

Rules:

- `designJsonVersion` is an integer. Within v1 only additive optional fields are allowed; consumers must ignore unknown fields. Renames/removals require v2 plus a converter. JSON Schema lives at `schemas/design-json.v1.json` and is CI-validated against the TS types.
- `box` is page-absolute; rotation is radians clockwise about the box center (02 §5); `parentId` and `children[]` preserve the graph. `layout` preserves `autoLayout` semantics, while solved geometry is authoritative for fidelity checks. Importers must re-solve layout and use the stored box only when `layout.mode === "NONE"`.
- Tokens are included by variable name path and ID, with aliases preserved. `components` carries `ComponentDefinition` metadata, variant axes, and property definitions; `interactions` carries 02 §12 records with destination IDs preserved.
- Round-trip import (same file or new file) restores nodes, tokens, components, and interactions when IDs do not collide. Collisions remap IDs and record `W_ID_REMAPPED`; unresolvable references become dangling markers (02 §18.1), never deletions. Plugin data outside `metadata.pluginData` is not round-tripped.
- The schema is intentionally lossy about editor state (selection, zoom, history, comments). Export metadata records `planHash`, theme, and diagnostics so consumers can detect approximations.

### 4.2 W3C Design Tokens

| Source entity | `$type` | `$value` encoding |
|---|---|---|
| `Variable` `COLOR` | `color` | `#rrggbb`/`#rrggbbaa`; exact RGBA in `$extensions` |
| `FLOAT` category `space`, `size`, `radius`, `border` | `dimension` | `{ "value": 16, "unit": "px" }` |
| `FLOAT` category `opacity`, `elevation` ratio | `number` | 0..1, 4 decimals max |
| `FLOAT` typography `fontSize`, `paragraphSpacing` | `dimension` | px |
| `lineHeight` `PIXELS` / `PERCENT` | `dimension` / `number` | `{value,unit:"px"}` / ratio (`140%` → `1.4`) |
| `letterSpacing` | `dimension` | px, or `em` for percent |
| `STRING` | `string` / `fontFamily` | literal; no interpolation |
| `BOOLEAN` | `boolean` | literal |
| `Style` `EFFECT` | `shadow` | object or array; `inset: true` for `INNER_SHADOW` |
| `Style` `TEXT` | `typography` | family, numeric weight, size, line height, spacing |
| `Style` `FILL` with gradient | `gradient` (draft) or CSS string | stops + `gradientTransform` in `$extensions` |
| Motion variables | `duration` | `{ "value": 200, "unit": "ms" }` |

- Names map `/` → `.` in DTCG and `-` in CSS. `color/bg/default` → `color.bg.default` / `--color-bg-default`. Normalization collisions are errors (`E_TOKEN_COLLISION`), never silent renames.
- Aliases use DTCG syntax `{color.bg.default}`. A `VariableAlias` that resolves outside the exported scope becomes a literal plus `W_TOKEN_ALIAS_RESOLVED`; alias depth follows 02 §9.1 (32).
- Every token carries `$extensions["dev.kanvas"]` with `variableId`, `collectionId`, `modeId`, `scopes`, and `sourceName`. This is the round-trip anchor; numeric normalization never loses the original.
- Output files: `tokens/core.json` (default theme, aliases preserved), `tokens/themes/{slug}.json` (only values that differ from the default, fully resolved), optional `tokens/resolved/{slug}.json`. The default theme is `FileMeta.defaultThemeId`, else `Theme.isDefault`, else the first theme by order; no theme is `E_THEME_REQUIRED`. Maximum 16 themes and 5,000 variables per export.

```json
{ "color": { "$type": "color", "bg": { "default": {
  "$value": "{color.surface.default}", "$description": "Default canvas background",
  "$extensions": { "dev.kanvas": { "variableId": "v_0192", "collectionId": "col_theme", "scopes": ["FRAME_FILL"] } } } } } }
```

CSS custom properties use `:root` for the default theme and `[data-theme="<slug>"]` for each other theme. Aliases emit `var(--…)`; values use px, `em`, `%`, or unitless numbers, never locale-formatted strings. The Tailwind preset maps token groups to `theme.update` namespaces (`colors`, `spacing`, `borderRadius`, `boxShadow`, `fontSize`, `lineHeight`, `letterSpacing`, `opacity`, `borderWidth`), with values as `var(--…)`. TypeScript declarations are `as const` objects plus a `TokenPath` union; no runtime package.

```css
:root { --color-bg-default: var(--color-surface-default); --space-4: 16px; }
[data-theme="dark"] { --color-bg-default: var(--color-surface-inverse); }
```


### 4.3 Accessibility annotations

`A11yAnnotation` is a sidecar keyed by `nodeId` (02 §18.3 permits sidecars): `{ role, headingLevel (1–6), alt, decorative, label, description, focusOrder, landmark }`. Precedence: explicit sidecar, then `metadata.agentAnnotations` of kind `A11Y` with a strict `key:value` grammar, then heuristics only when `semanticMode: 'INFER'`. Default export never guesses a heading or form role.

- Alt text is required for `IMAGE`; `decorative: true` emits `alt=""`/`aria-hidden`, missing alt is `E_A11Y_ALT` in strict mode. Generated markup never invents labels from adjacent text.
- Heading levels emit exactly as annotated; skips (`h2` → `h4`) warn, never silently demote. One `h1` per exported page warns on duplicates.
- Focus order uses explicit `focusOrder`, then DOM/tree order for interactive nodes. Positive `tabindex` is never emitted; duplicates or gaps warn.
- The contrast report implements WCAG 2.2 AA (4.5:1 normal, 3:1 large/non-text; AAA optional). It composites ancestor backgrounds and alpha; image/gradient backdrops are `UNKNOWN`, not guessed. Output: `a11y-report.json` + `a11y-report.md`, with nodeId, pair, ratio, threshold, and verdict.

### 4.4 Limits and edge cases

- DesignJSON caps at 50,000 nodes and 100 MB canonical JSON; assets above 10 MB are linked, not embedded.
- Token export supports one theme set at a time; multi-collection themes are flattened by 02 §9.1. Hidden variables (`hiddenFromPublishing`) are excluded unless `includeHidden`.
- Composite text styles with unknown `fontStyle` weights warn and use the registry's OS/2 weight, else 400.
- Contrast over images/gradients is reported `UNKNOWN`; no automated pass/fail is fabricated.
- Import of a v2 document into a v1 consumer fails with `E_SCHEMA_VERSION`, never partial import.

### 4.5 Acceptance criteria

1. `design.json` validates against the published v1 schema and re-imports with node/token/component counts unchanged on the fixture set.
2. Exporting the token fixture twice is byte-identical and preserves every alias in `core.json`.
3. CSS/Tailwind/TS token outputs resolve to the same computed values as `tokens/resolved/*.json`.
4. A theme switch changes only theme files and resolved values, not token names or aliases.
5. Accessibility export produces a contrast report with no `UNKNOWN` verdicts for solid-on-solid text in the `a11y-form` fixture, and flags the known failures.

---

## 5. Code Generation — React + Tailwind (Primary Target)

Emitter `react-tailwind`. Defaults: TypeScript, function components, Tailwind 3.4 preset plus CSS variables, `data-theme` dark mode, Prettier 3, no runtime styling library, no remote assets.

### 5.1 Node mapping

| 02 construct | React output | Rules |
|---|---|---|
| `FRAME` with `autoLayout` | `<section>`/`<div>` + flex classes | Element from annotation (`landmark`), else `div`; `clipsContent` → `overflow-hidden` |
| `FRAME` without `autoLayout` | `relative` container | Children with `constraints` are absolutely positioned; root may be `min-h-dvh` |
| `GROUP` | `<div>` | No visual styles unless opacity/effects; never use `display: contents` (breaks transforms) |
| `RECTANGLE` | `<div>` | Background/border/radius; a 1 px divider may become `<hr>` only with an annotation |
| `ELLIPSE` | `<div>` `rounded-full` | `arcData` → inline SVG path |
| `VECTOR`, `BOOLEAN` | inline `<svg>` | Precomputed path, `viewBox="0 0 w h"`, `fillRule`; icons default `aria-hidden` |
| `LINE`, `ARROW` | inline `<svg>` | Stroke, dash, caps; arrow markers expanded to paths |
| `TEXT` | `h1`–`h6`, `p`, `span`, `label`, `a` | Chosen by `A11yAnnotation`; default `p`; hyperlink → `<a>` after scheme validation |
| `IMAGE` | `next/image` or `<img>` | Local asset import; explicit width/height; required `alt` |
| `INSTANCE` | imported component | Props from `componentProperties` + `overrides`; detached → inline subtree |
| `COMPONENT` / `COMPONENT_SET` | component file + variant map | Masters are not rendered as children unless explicitly scoped |
| `SLICE`, `SECTION` | omitted | Slice affects crop only; section exports descendants |

### 5.2 Layout and style emission

| Design property | Tailwind / CSS |
|---|---|
| `direction: HORIZONTAL/VERTICAL` | `flex flex-row` / `flex flex-col` |
| `wrap: WRAP` | `flex-wrap`; `counterSpacing` → `gap-y-*` with `gap-x-*` |
| `itemSpacing` | `gap-[Npx]`; token-bound → `gap-{token}` |
| `padding` | `p-*`, `px-*`, `py-*`, or `pt/pr/pb/pl-*`; per-side values stay separate |
| `primaryAxisAlign` | `justify-start`/`justify-center`/`justify-end`/`justify-between` |
| `counterAxisAlign` | `items-start`/`items-center`/`items-end`/`items-baseline` |
| `sizing: HUG` | `w-fit`/`h-fit`; text `WIDTH_AND_HEIGHT` → `w-fit whitespace-nowrap` |
| `sizing: FILL`, `layoutGrow: 1` | `flex-1` (primary) / `self-stretch` (`layoutAlign: STRETCH`) |
| `layoutPositioning: ABSOLUTE` | `absolute left-[Xpx] top-[Ypx]`; parent `relative` |
| `constraints` (non-auto-layout parent) | `LEFT/RIGHT/LEFT_RIGHT/CENTER/SCALE` → `left-*`, `right-*`, `inset-x-*`, `left-1/2 -translate-x-1/2`, percentage insets/width |
| `cornerRadius`, `clipsContent` | `rounded-*`, `overflow-hidden` |
| `effects` | `shadow-*`, `shadow-[inset_…]`, `blur-*`, `backdrop-blur-*` |
| `opacity`, `blendMode` | `opacity-*`, `mix-blend-*`; `PASS_THROUGH` emits nothing and warns if isolated |
| `rotation`, negative scale | `rotate-[deg]`, `-scale-x-100`/`-scale-y-100`; origin center |

Style emission is token-first. A bound `color/bg/surface` becomes `--color-surface` + `colors.surface` + `bg-surface`; `space/4` → `p-4`; `radius/lg` → `rounded-lg`; `shadow/card` → `shadow-card`; `typography/heading/md` → `text-heading-md`. Unbound values use arbitrary utilities (`p-[18px]`, `text-[15px]`, `bg-[#f7f7f8]`) and emit `W_HARDCODED_VALUE`. Inline `style` is allowed only for runtime values (measured geometry, user-provided CSS vars); static geometry uses utilities. Classes are emitted in canonical buckets (layout, box, typography, color, effects, states, responsive) so output does not depend on a formatter plugin.

Responsive output is `responsiveMode: 'NONE' | 'FRAME_WIDTH' | 'BREAKPOINT_SET'`. `FRAME_WIDTH` turns a fixed frame into `w-full max-w-[Wpx]`; `BREAKPOINT_SET` groups same-base-name frames into one component with `sm:`/`md:`/`lg:` rules (detected width bands 640/768/1024/1280/1536, opt-in). Hover/focus variants are derived only when a `COMPONENT_SET` state variant has the same tree signature and differs solely in paints/strokes/effects; then `hover:`/`focus-visible:` classes are emitted. Structural state changes require a client component and are emitted only with `interactions: 'FULL'`.

### 5.3 Component extraction

- Signature = structural hash of type, normalized name, layout, size (rounded), token bindings, and child signatures; text content, instance props, and IDs are excluded. Repeats ≥3 with ≤300 nodes become a component; occurrences inside a component do not nest a second component.
- Props: varying `TEXT` → string prop (`label`, `title`, `description`); `BOOLEAN` property → boolean conditional; `INSTANCE_SWAP` → `ReactNode` slot or `ComponentType`; `VARIANT` axis → string union with the default variant; `overrides` with `when` conditions become variant-scoped lookups when possible, otherwise `W_CONDITIONAL_OVERRIDE`.
- Names: `ComponentDefinition.name` → PascalCase (`Button/Primary` → `ButtonPrimary`), reserved words and leading digits handled, non-Latin names fall back to `Component<hash6>`. File names are kebab-case.
- Output: `src/components/{Name}/{Name}.tsx`, `{Name}.types.ts`, `index.ts`; `src/design/tokens.css`, `src/design/tailwind.preset.ts`, `src/lib/cn.ts`, `src/assets/*`. `index.ts` re-exports only public components and types. Imports are sorted external → alias → relative → CSS.

### 5.4 Code quality rules

- Prefer semantic HTML: one `main`, headings in order, `<button>` for actions, `<a>` for navigation, `<label>` wrapping inputs or `htmlFor` with a stable `useId`. Never turn a `TEXT` into a clickable `div` when a button role is annotated.
- No `dangerouslySetInnerHTML`. Escape JSX text (`&`, `<`, `>`), attribute quotes, backticks, and `${` in template literals. Validate URLs to `https:`, `http:`, `mailto:`, `tel:`; reject `javascript:`.
- Stable keys only when mapping dynamic arrays; static children are emitted literally, never `.map`.
- Assets are local imports; no `fetch`, no remote URLs, no runtime image transformation. Fonts are local `@font-face` assets only when licensing allows (§2.5).
- TypeScript `strict`; no `any`; `import type`; props interfaces exported. `"use client"` only when hooks, event handlers, or prototype state require it.
- Output is formatted with pinned Prettier (printWidth 100, singleQuote, trailingComma `all`) and must be ESLint-clean under `jsx-a11y` recommended rules. Warnings are emitted as `// TODO(kanvas): …` only when the user opted into partial output; otherwise the job fails.

### 5.5 Worked example: login card

Input: the `n_card` subtree from 02 §17.1, theme Light, annotations marking the card as a form landmark, the title as `h2`, and the two field rectangles as email/password inputs.

```tsx
// src/components/LoginCard/LoginCard.tsx
// Generated by kanvas react-tailwind@1.0.0. Source revision 4821.
import type { FormEvent } from "react";
import { Button } from "@/components/Button";

export interface LoginCardProps {
  title?: string;
  emailLabel?: string;
  passwordLabel?: string;
  submitLabel?: string;
  footer?: string;
  onSubmit?: (event: FormEvent<HTMLFormElement>) => void;
}

export function LoginCard({
  title = "Selamat datang kembali",
  emailLabel = "Email",
  passwordLabel = "Kata sandi",
  submitLabel = "Masuk",
  footer = "Belum punya akun? Daftar",
  onSubmit,
}: LoginCardProps) {
  return (
    <section
      aria-label={title}
      className="flex min-h-dvh w-full flex-col items-center justify-center bg-surface px-6 py-12"
    >
      <form
        className="flex w-full max-w-[342px] flex-col gap-4 rounded-2xl bg-surface-raised p-6 shadow-card"
        noValidate
        onSubmit={onSubmit}
      >
        <h2 className="text-heading-md text-text-primary">{title}</h2>
        <label className="flex flex-col gap-2">
          <span className="text-label-md text-text-secondary">{emailLabel}</span>
          <input
            autoComplete="email"
            className="h-12 w-full rounded-lg border border-border-default bg-surface-sunken px-3 text-body-md text-text-primary placeholder:text-text-tertiary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            inputMode="email"
            name="email"
            placeholder="nama@perusahaan.com"
            required
            type="email"
          />
        </label>
        <label className="flex flex-col gap-2">
          <span className="text-label-md text-text-secondary">{passwordLabel}</span>
          <input
            autoComplete="current-password"
            className="h-12 w-full rounded-lg border border-border-default bg-surface-sunken px-3 text-body-md text-text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            name="password"
            required
            type="password"
          />
        </label>
        <Button className="mt-2 w-full" type="submit" variant="primary">
          {submitLabel}
        </Button>
        <a
          className="text-center text-label-md text-accent underline-offset-4 hover:underline"
          href="/signup"
        >
          {footer}
        </a>
      </form>
    </section>
  );
}
```

Token values are shipped as CSS variables; the dark theme only swaps values, never class names:

```css
/* src/design/tokens.css */
:root {
  --color-surface: #ffffff;
  --color-surface-raised: #ffffff;
  --color-surface-sunken: #f7f7f8;
  --color-text-primary: #15161a;
  --color-text-secondary: #5b5f6b;
  --color-text-tertiary: #8a8f9c;
  --color-border-default: #d9dbe1;
  --color-accent: #4f46e5;
  --radius-lg: 8px;
  --radius-2xl: 16px;
  --shadow-card: 0 8px 24px rgb(0 0 0 / 0.08);
}
[data-theme="dark"] { --color-surface: #121318; --color-surface-raised: #1b1d24; }
```

### 5.6 Limits and edge cases

- Max 5,000 nodes per code job, 300 nodes per extracted component, 32 levels of nesting, 200 components; exceeding these keeps markup inline and emits `W_COMPONENT_LIMIT`.
- Unsupported constructs (complex masks, `BACKGROUND_BLUR` without backdrop, `AI_GENERATE`) emit a diagnostic and a typed TODO; `strict` turns them into failures.
- Fixed-height text with substituted fonts is emitted with `overflow-hidden`; the mismatch is reported, never silently accepted as pixel-faithful.
- Multiple themes export once per theme only when `themeMode: 'MULTI'`; otherwise the selected theme is baked into CSS vars and `W_THEME_BAKED`.

### 5.7 Acceptance criteria

1. The generated login fixture compiles under TypeScript strict, passes ESLint + jsx-a11y, and renders without runtime warnings in Playwright.
2. No generated file contains inline `style` except values proven dynamic by the fixture.
3. Every color/spacing/radius/typography value bound to a token is emitted as a token utility or CSS var; unbound values are arbitrary utilities with a warning.
4. Re-running codegen on the same revision and options is byte-identical, including import and class order.
5. Keyboard focus order and accessible names match the `a11y-form` expectations.

---

## 6. Code Generation — Other Targets

### 6.1 HTML + CSS and Tailwind HTML

`html-css` emits semantic HTML plus one `styles.css`; class names are kebab-case layer names with a 10-hex suffix on collision, not generated hashes, so the markup stays readable. Auto layout maps to flex, absolute children keep `position:absolute`, and tokens emit as CSS custom properties exactly as in §4.2. `tailwind-html` emits the same markup with utility classes and a `tailwind.config.js` preset; it shares the React mapping in §5.1 minus component extraction and event props. Both emit `index.html`, `assets/`, `tokens.css`, and optional `prototype.js` only when `interactions: 'FULL'`.

Limits: no component state, no framework lifecycle, no bundler; complex masks are rasterized to PNG; `INSTANCE` overrides are flattened. Inline `style` is restricted to measured absolute geometry.

### 6.2 React Native

| Design | React Native |
|---|---|
| Frame + `autoLayout` | `<View style={{ flexDirection, gap, padding, alignItems, justifyContent }}>` |
| `HUG` / `FILL` / `layoutGrow` | `alignSelf: 'flex-start'`, `flex: 1`, `flexGrow: 1`; parent needs a bounded cross size |
| `layoutPositioning: ABSOLUTE` | `position: 'absolute', left, top` |
| `TEXT` | `<Text>` only; rich ranges are nested `<Text>`; `numberOfLines` + `ellipsizeMode` for truncation |
| `IMAGE` | `<Image source={require(...)} resizeMode="cover|contain|repeat" />`; local assets only |
| `VECTOR` / `BOOLEAN` | `react-native-svg` `<Svg><Path /></Svg>` |
| Drop shadow | iOS `shadowColor/Offset/Opacity/Radius`; Android `elevation`; inner shadow unsupported |
| `LAYER_BLUR` / `BACKGROUND_BLUR` | `expo-blur` only (`runtime: 'EXPO'`); otherwise unsupported |
| Gradients | `expo-linear-gradient` only; radial/angular/diamond unsupported |
| Fonts | `fontFamily` from bundled assets; `allowFontScaling` and `maxFontSizeMultiplier` recorded |

StyleSheet objects are emitted per component, not inline, sorted by key. `gap` requires RN ≥ 0.71; `runtime: 'BARE'` disables Expo-only effects. Safe areas use `react-native-safe-area-context` when the frame is a prototype root. Blend modes, CSS masks, and arbitrary filters are unsupported; each emits a diagnostic. Interactions map to `Pressable`/`onPress`; navigation is a caller-provided adapter.

### 6.3 Flutter

| Design | Flutter |
|---|---|
| `autoLayout` | `Row`/`Column` (+`Wrap` for `wrap: WRAP`); `mainAxisAlignment`, `crossAxisAlignment` |
| Spacing/padding | `SizedBox`/`Spacer` + `EdgeInsets.fromLTRB` |
| Absolute children | `Stack` + `Positioned`; root uses `SizedBox`/`ConstrainedBox` |
| `RECTANGLE`/`ELLIPSE` | `Container`/`DecoratedBox` with `BoxDecoration`, `BorderRadius`, `BoxShadow` |
| Gradients | `LinearGradient`, `RadialGradient`, `SweepGradient`; diamond approximated |
| `IMAGE` | `Image.asset(..., fit: BoxFit.cover)` |
| `VECTOR`/`BOOLEAN` | `SvgPicture.asset` (`flutter_svg`) or `CustomPainter` |
| Blur | `ImageFiltered` for layer blur; `BackdropFilter` for background blur |
| Masks | `ClipPath`/`ClipRRect`; complex alpha masks rasterized |
| Text | `Text`/`RichText`, `TextStyle`, `maxLines`, `overflow: TextOverflow.ellipsis` |
| Tokens | generated `lib/theme/tokens.dart` (`AppTokens` class), no CSS variables |

Inner shadows require a `CustomPainter` or raster fallback. Blend modes use `BlendMode` where Flutter supports them (`multiply`, `screen`, `overlay`, `darken`, `lighten`); hue/saturation/color/luminosity are unsupported. Fonts are declared in `pubspec.yaml`; output includes the snippet. Constraints must be bounded before `Expanded`/`Spacer`; unbounded cases emit `W_FLUTTER_UNBOUNDED`.

### 6.4 Vue and Svelte (roadmap)

`@kanvas/emitter-vue` and `@kanvas/emitter-svelte` are roadmap plugins implementing the §8 interface only; no v1 code path registers them. They advertise `capabilities.stable: false`, reuse `ExportPlan`, and may not alter the plan schema. Until shipped, `.vue`/`.svelte` output is explicitly unavailable.

### 6.5 Per-target fidelity matrix

| Construct | PNG/JPG/WebP | SVG | PDF | React+TW | HTML+CSS | RN | Flutter |
|---|---|---|---|---|---|---|---|
| Linear/radial gradient | Exact | Exact | Exact | Exact | Exact | Expo only | Exact |
| Angular/diamond gradient | Exact | Raster | Raster | Conic exact / diamond approx | Same | None | Sweep / diamond approx |
| Drop/inner shadow | Exact | Native filters | Rasterized | Exact | Exact | Approx (elevation) | Exact / inner approx |
| Layer blur | Exact | Filter | Rasterized | CSS filter | CSS filter | Expo blur only | `ImageFiltered` |
| Background blur | Exact | Rasterized | Rasterized | `backdrop-blur` | `backdrop-filter` | Expo blur only | `BackdropFilter` |
| Blend modes | Exact | CSS mix-blend | PDF 1.7 | CSS | CSS | None | Partial |
| Masks | Exact | Exact | Exact | CSS/simple, complex raster | Same | SVG clip only | `ClipPath` |
| Boolean ops | Exact | Path result | Path result | Inline SVG | Inline SVG | `react-native-svg` | `flutter_svg` |
| Vector networks | Exact | Exact | Exact | Inline SVG | Inline SVG | `react-native-svg` | `flutter_svg` |
| Text truncation | Exact | Pre-truncated | Exact | `line-clamp` | `line-clamp` | `numberOfLines` | `maxLines` |
| Auto-layout wrap | Exact | Flattened | Flattened | `flex-wrap` | `flex-wrap` | `flexWrap` | `Wrap` |
| Variants | Selected only | Selected only | Selected only | Typed props | CSS classes | Props | Constructor/enum |

### 6.6 Limits and edge cases

- Every non-primary target is presentational. No target generates data fetching, validation, auth, analytics, persistence, backend calls, or secrets.
- RN/Flutter assets are local and declared in the package; missing platform support is a diagnostic, not a silent omission.
- Flutter output targets stable Dart 3; RN targets TypeScript + RN 0.71+; HTML targets evergreen browsers.
- Vue/Svelte are roadmap and must not appear in `ExportTarget` or be offered by the UI (04) until shipped.

### 6.7 Acceptance criteria

1. React, HTML, and Tailwind HTML fixtures compile/open with no console errors and pass the §11 E2E checks.
2. RN and Flutter fixtures build in CI (Expo prebuild / `flutter build web` smoke) for the supported set.
3. The fidelity matrix is enforced by tests that assert a diagnostic for every `Approx`/`None` cell.
4. Output contains no unsupported import, remote asset URL, or target-specific API outside its declared runtime.

---

## 7. Interaction & Prototype Code Export

Interactions are optional (`interactions: 'NONE' | 'NAVIGATE' | 'FULL'`). `NAVIGATE` emits routing and links; `FULL` adds state and motion. Generated code is a starting point, not a runtime for the editor.

| 02 interaction | Generated output |
|---|---|
| `NAVIGATE` / `BACK` | `<Link>`/`router.push` (Next App Router) or adapter; `BACK` → `router.back()` |
| `OPEN_LINK` | `<a href>` when it is the only action; otherwise validated `window.open` with `rel="noopener noreferrer"` |
| `SCROLL_TO` | `ref.current?.scrollIntoView({ behavior, block: 'start' })`; offset applied via `scroll-margin-top` |
| `SET_VARIABLE` | `useState`/context reducer keyed by variable ID; theme-mode changes set `data-theme` |
| `CHANGE_VARIANT` / `SET_STATE` | state setter + typed lookup; no direct DOM mutation |
| `AFTER_TIMEOUT` | `useEffect` timer with cleanup; never runs during SSR |
| `ON_CLICK` / `ON_PRESS` / `ON_HOVER` | semantic handler or CSS `hover:`/`focus-visible:` when style-only |
| `ON_DRAG` / `ON_SCROLL` / `ON_KEY` | event listener with threshold; best-effort, diagnostic if complex |
| `OPEN_OVERLAY` (via `NAVIGATE` + `overlay`) | `<dialog>` with focus trap; framer-motion optional for transitions |
| `AI_GENERATE` / `AI_VARIANT` | never emitted; typed TODO + `W_AI_RUNTIME_ACTION` |

Transitions map `INSTANT`/`DISSOLVE`/`SLIDE_*`/`MOVE_*` to CSS or optional `framer-motion`; `SMART_ANIMATE` tweens only transform and opacity and only for pairs matched by 02 §12. Auto-layout reflow, color cross-fades between gradients, and unmatched structural changes are not animated. Spring specs map to `motion` spring values; unsupported easing falls back to `cubic-bezier` with a warning.

Not generated: business logic, API calls, authentication, authorization, payments, analytics, data persistence, form validation beyond HTML constraints, and any server code. `onSubmit` is a prop with no default implementation. Prototype cycles are preserved; unreachable frames warn; overlay focus restoration is included only when `FULL`.

Limits: max 1,000 interactions per job; cross-file destinations are dropped with `W_LINK_EXTERNAL`; runtime AI actions always require a human implementation. Acceptance: navigation fixture renders every link with the correct href/route; keyboard activation works; `smart-animate` fixture produces only transform/opacity transitions and a diagnostic for every dropped property.

---

## 8. Emitter Plugin API

```ts
interface Emitter {
  name: string;                         // "react-tailwind", globally unique
  version: string;                      // semver; part of every cache key
  targets: ExportTarget[];
  capabilities: {
    stable: boolean;                    // false = roadmap/preview, hidden by default
    streaming: boolean;                 // supports chunked packaging
    deterministic: boolean;
    needs: ('FONTS' | 'RASTER' | 'PDF' | 'NETWORK')[];
  };
  optionsSchema: Record<string, unknown>;   // JSON Schema 2020-12
  emit(plan: ExportPlan, options: EmitterOptions, ctx: EmitterContext): Promise<EmitterResult>;
}

interface EmitterContext {
  signal: AbortSignal;
  reportProgress(stage: ExportStage, completed: number, total: number): void;
  format(path: string, contents: string): Promise<string>;  // pinned Prettier/format hooks
  hash(bytes: Uint8Array): string;
  logger: { info(code: string, msg: string): void };
}

interface EmitterResult {
  files: EmittedFile[];                 // path, UTF-8/binary contents, mode
  warnings: Diagnostic[];
  metadata: { deterministic: boolean; toolchain: Record<string, string> };
}
```

Registration is explicit at worker boot (`registerEmitter`), with built-ins in `packages/export/emitters`; unknown targets fail `E_TARGET_UNKNOWN`. Options are validated with Ajv against `optionsSchema`; unknown keys are errors, defaults are canonicalized before hashing. Emitters run in a worker sandbox with no network, no direct filesystem, a 1.5 GB memory cap, and CPU timeouts; built-ins use the same contract as third-party plugins. Third-party remote code loading is roadmap; when enabled it must run in an isolated worker with declared permissions, not `eval` in the main process.

Versioning is semver. Any output-affecting change bumps at least minor; breaking option/schema changes bump major. The cache key uses the full version (§9.2), so patch releases cannot silently reuse old output. Deprecated emitters remain registered for one minor cycle with `deprecationNotice`.

### 8.1 Agent invocation

The agent tool is `kanvas.export_code` (registry and permissions in 05; UI in 04). Input: `{ fileId, pageIds?, nodeIds?, target, options?, themeId?, dryRun?, idempotencyKey? }`. It requires the `export` scope and `EXPORT_RUN` capability, writes an `AuditLogEntry` with `kind: 'EXPORT'`, and returns a job ID plus, for `dryRun: true`, a plan summary, file tree, unified diff against the last successful export of the same scope, and `estimate: { ms, bytes, artifacts, costUsd: 0 }`. Dry-run never persists artifacts. Full output is capped at 200 KB inline; larger results return signed URLs. The tool must state warnings and cannot hide `deterministic: false`.

Limits: one target per call; third-party emitters are not agent-invocable until sandboxing ships; diffs are line-based and do not include binary assets (hashes only). Acceptance: the tool rejects missing `export` scope, returns identical hashes for repeated calls, and never exposes another file's artifacts.

---

## 9. Performance & Scale

Budgets are p95 on the reference worker (8 vCPU, 16 GB, warm font/asset cache, pinned Chromium) or an M2-class laptop for client budgets. Cold starts add up to 300 ms.

| Workload | Budget | Notes |
|---|---|---|
| PNG, 10k-node export scope | < 3 s server | Multi-frame flow; 02 §18.1 still caps a single frame at 2,000 nodes |
| SVG, 2k nodes | < 500 ms client | Pure serializer; 1 MB output target |
| Codegen, 500 nodes | < 1.5 s | React/Tailwind, Prettier included |
| PDF, 10 frames | < 4 s server | Font subsetting included |
| Tokens, 5,000 variables | < 800 ms | All themes |
| Sprite, 256 icons | < 1 s server | 4,096 px atlas cap |
| Worker memory | < 1.5 GB | Chromium recycled every 200 renders/10 min |
| Client memory | < 512 MB | Above this, jobs move to the worker |

The plan build is O(nodes) after memoized intrinsic layout (02 §7.1). Emitters stream where possible; ZIP is written with data descriptors, CRC computed incrementally, and no full-archive buffering. Raster tiles render sequentially per frame but may parallelize across frames up to 2 workers; output order is fixed by scope order, never completion order.

Caching is content-addressed: `sha256(planHash + emitter.name@version + canonicalJSON(options) + rendererBuild + fontHashes + formatterVersion)`. `planHash` covers revision, scope roots, theme, resolved token values, and asset hashes. Any change invalidates the entry; emitter patch versions invalidate conservatively. Entries expire after 7 days and are garbage-collected with their artifacts. Cache hits skip stages 6–8 and re-sign the stored artifact; the manifest still reflects the original plan and renderer.

Limits: budgets are not SLOs for over-limit imported files; jobs above 50k nodes are rejected, and `E_TIMEOUT` is preferred over unbounded memory. Acceptance: the `large-10k` fixture meets the raster budget on reference hardware, a cache hit returns in < 150 ms, and changing any one cache dimension produces a new key.

---

## 10. Security & Compliance

| Threat | Mitigation |
|---|---|
| Malicious SVG (script, `foreignObject`, external refs, entity expansion) | Allowlist sanitizer at import and export; strip `<script>`, `on*`, external `href`, CSS `url()`/`@import`; reject `<!DOCTYPE>`; caps: 5 MB, 100k path commands |
| ZIP path traversal / zip-slip | Reject `..`, absolute paths, backslashes, NUL/control chars, symlinks; NFC-normalize; max 180 chars; duplicate entries error |
| Generated-code injection | Escape JSX text, attributes, template literals; sanitize identifiers; validate URLs; no `eval`, `Function`, or `dangerouslySetInnerHTML`; JSON embedded with `<` escaped |
| Font/asset license violation | License sidecars (§2.5); no binary embedding without `ALLOWED`; manifest lists all licenses and attribution |
| PII in screenshots/exports | Private bucket, 15-minute signed URLs, audit log, `redactNodeIds` replaces nodes with solid blocks; UI warns before exporting user-content frames (04) |
| Cross-tenant artifact access | RLS on jobs/artifacts by file/workspace; signed URLs bound to one object; agents need `export` scope |

Exports are immutable once published: a new revision or option creates a new job/artifact, never an in-place replacement. `AuditLogEntry` records requester, target, scope, plan hash, and artifact hashes, but not file contents. ZIPs are scanned for forbidden path patterns after writing and before delivery.

Limits: sanitization is defense in depth, not a renderer sandbox; untrusted SVG is parsed into `VectorPath` before it reaches a renderer. PII detection is not automatic; `redactNodeIds` is explicit and itself recorded. Acceptance: the malicious-SVG fixture produces `E_ASSET_UNSAFE` with no network request; a zip-slip fixture is rejected; the license fixture never embeds a restricted font; an agent without `export` scope gets `E_FORBIDDEN`.

---

## 11. Testing Strategy

Fixtures are canonical `.kanvas` files built by the fixture package and committed with expected outputs: `core-login`, `tokens-multimode`, `effects-zoo`, `vector-boolean`, `a11y-form`, `rtl-arabic`, `large-10k` (over-limit stress), `malicious-svg`, and `asset-licenses`. Each target commits golden files under `packages/export/__fixtures__/{fixture}/{target}/`, including `manifest.json`.

| Test class | Scope |
|---|---|
| Golden bytes | Every deterministic target; CI compares SHA-256 and fails on any difference |
| Property | Determinism (repeat, shuffled keys, TZ/locale, worker count), idempotent token export, plan-hash stability, path sanitizer, ZIP round-trip |
| Raster visual | Pinned renderer; ≤ 0.05% pixels with channel delta > 8 and no 64×64 tile above 1% differing; exact dimensions |
| SVG structural | Parse XML, canonicalize attribute order, compare elementtree; assertions for `path d`, clip/filter refs, IDs |
| PDF structure | Page count/size, font subsets, `ToUnicode`, link annotations, extracted text |
| Code | TypeScript/Flutter compile, ESLint + jsx-a11y, Playwright render + keyboard navigation |
| E2E | Select frame → export dialog → job progress → download → unzip → manifest validation; cancel and retry paths |

CI matrix: Linux x64 only for golden raster (renderer determinism), Node 22, pnpm lockfile, Chromium pinned; macOS smoke for client Konva only. Visual and byte goldens are separate jobs so a renderer bump is visible. Snapshot updates require `UPDATE_SNAPSHOTS=1`, a diff summary, visual attachments for raster changes, and review by a design-systems owner; blind updates are blocked by a CODEOWNERS rule. Emitter behavior changes require a changelog entry and a version bump.

Limits: property tests use small generated documents (≤ 500 nodes); full 10k fixtures run nightly, not on every PR. Acceptance: a deliberate 1 px geometry change fails at least one golden per target; a shuffled input object produces zero diff; the E2E cancel test leaves no artifact.

---

## 12. Acceptance Criteria

These gate v1. Each is testable in CI or by a documented manual check; roadmap targets are exempt.

1. Determinism: identical plan/options/toolchain produce byte-identical files and ZIPs.
2. PNG/JPG/WebP honor scale 0.5–4, DPR independence, alpha rules, sRGB, and tiling caps.
3. SVG maps every 02 node type, namespaces IDs, declares accessibility, and documents raster fallbacks.
4. PDF preserves vectors and selectable subset text, sizes pages, and links in-scope frames.
5. Sprites/icon presets are deterministic and follow naming, collision, ZIP, and manifest rules.
6. `DesignJSON` validates against its schema and round-trips nodes, tokens, components, and interactions.
7. DTCG output preserves aliases, modes, descriptions, extensions, and deterministic ordering.
8. CSS/Tailwind/TS token outputs compute to the same values as resolved token JSON.
9. A11y export covers alt text, headings, focus order, and a contrast report with no fabricated verdicts.
10. React/Tailwind output compiles, lints, is keyboard-accessible, and uses tokens before arbitrary values.
11. HTML/RN/Flutter outputs compile or open and respect their documented fidelity limits.
12. Interactions generate navigation/state only; no business logic, fetching, auth, or runtime AI calls.
13. Emitter options validate, versions invalidate caches, and sandbox restrictions are enforced.
14. Budgets in §9 are met on reference hardware; over-limit jobs fail with actionable errors.
15. Security tests reject malicious SVG, zip traversal, code injection, and unlicensed font embedding.
16. All golden, property, visual, structural, and E2E suites pass; snapshot updates follow §11 review.

Limits: criteria apply to the targets in §1; Vue/Svelte, AVIF, vector PDF effects, tagged PDF, and animation codegen are not covered. Verification is the CI matrix plus a release checklist signed by the export owner.

---

## 13. Open Questions / Deferred

- Figma `.fig` import and full fidelity round-trip are out of scope; `DesignJSON` import is the supported interchange path.
- Animation codegen beyond `SMART_ANIMATE` transform/opacity and CSS transitions is deferred.
- Vector PDF effects (native soft shadows/blurs) remain roadmap until goldens match raster fallbacks.
- Vue/Svelte emitters, AVIF, Android adaptive icons, iOS asset catalogs, and tagged PDF/PDF-UA are deferred.
- Token round-trip conflicts (DTCG import into an existing collection) need a merge policy; v1 reports conflicts and never overwrites.
- Multi-file/workspace export, server-side scheduled exports, and webhooks are deferred.
- The 10k-node raster budget conflicts with 02's 2,000-node frame cap; decide whether enterprise files may raise the cap before promising per-frame numbers.
- Third-party emitter sandboxing (permissions, signing, marketplace) is roadmap.

Limits: these items must not be advertised as shipped in UI (04) or agent tools (05). Done when each has an owner, a target release, and a spec amendment; until then, export rejects unsupported targets with a typed error.

---
