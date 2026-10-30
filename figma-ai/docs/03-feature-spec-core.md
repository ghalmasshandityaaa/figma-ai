# 03 — Core Editor Feature Specification
> **Status:** Normative for editor behavior. Visual design lives in `01-ux-architecture.md`.
> Identifiers are defined in [`02-data-model.md`](./02-data-model.md); additions proposed here are marked
> **[proposed]** and listed in §14.3.

## 1. Scope & How to Read
This document specifies the human editing surface: canvas and viewport, selection and direct
manipulation, shape/frame tools, pen and vector editing, text, auto layout UX, layer panel, pages,
inspect panel, search, and keyboard/accessibility behavior. Components, tokens UI, assets, prototyping,
comments, persistence, and agent runtime are specified in `04`, `05`, `07`, and `08`.

| Rule | Meaning |
|---|---|
| Normative words | **MUST** = required; **SHOULD** = default with documented deviation; **MAY** = optional |
| Units | CSS px at `zoom = 1` unless "screen px"; timings in ms; easings `cubic-bezier(0.2, 0, 0, 1)` |
| Precision | Store ≤ 3 decimals (02 §17); display ≤ 2; snap to integer px when pixel snapping is on |
| Mutation | UI emits `DesignCommand`s only; no component writes Zustand document state directly |
| Ephemeral | Viewport, hover, selection, tool, guides, and measurements are not DesignCommands |
| Verification | Every area has `AC-<section>.<n>` criteria in §13; test at 25%, 100%, 400% zoom and 10k nodes |

**Commands column convention.** `<namespace>.<verb>` follows 02. Ephemeral interactions say "none" and
must never appear in the history log. New commands are listed in §14.3.

**Client-owned state.** `EditorViewport { pageId, pan, zoom }` **[proposed]**, `NodeId[]` selection
(`PresenceState.selection`, max 50,000), session-only hover/entered-group/tool state, `Guide { id, axis,
position }` **[proposed]**, and `ClipboardDocument` **[proposed]**. Guides and per-page viewports are
per-user editor state, never shared, exported, or visible to agents.

---
## 2. Canvas & Viewport
### 2.1 Coordinate spaces
| Space | Definition | Used by |
|---|---|---|
| Page | Origin `(0,0)` at page top-left, +x right, +y down. Root transforms are page-relative (02 I7). | Document, comments, guides |
| Canvas | Identical to page space on the active page; no hidden offset. | Cursors, agent cursors, snaps, measurements |
| Screen | CSS px from the canvas element's top-left. | Hit tests, handles, overlays |
| Device | Physical px; `Konva.Stage.pixelRatio = min(devicePixelRatio, 2)`. | Rendering only |

`screen = (canvas - pan) * zoom`; `canvas = screen / zoom + pan`. `pan` is the canvas point at screen
`(0,0)`. Zoom is clamped to **0.02–64** (2%–6400%). World transforms compose ancestor transforms;
rotation is clockwise about the node center (02 §5). Never persist screen coordinates.

### 2.2 Pan and zoom
| Input | Behavior |
|---|---|
| `Space`+drag / middle-drag / `H` drag | Pan; `grabbing` cursor; no history |
| Two-finger scroll | Pan by gesture delta (`canvasDelta = screenDelta / zoom`) |
| Pinch / `Cmd/Ctrl`+wheel | Zoom around pointer; `exp(-deltaY * 0.0015)`; continuous |
| `Cmd/Ctrl +` / `Cmd/Ctrl -` | Next/previous preset around viewport center, 120 ms |
| `Shift+0` / `Shift+1` / `Shift+2` | 100% / fit page / focus selection (§2.3) |
| `Shift+Enter` | Center selection at current zoom |
| Double-tap trackpad | 100% at pointer |

Presets: **2, 5, 10, 25, 50, 75, 100, 150, 200, 300, 400, 800, 1600, 3200, 6400** (%). The zoom field
accepts 2–6400, `Fit`, and `Selection`. Pointer-anchored zoom must keep the canvas point under the
pointer within 0.5 screen px. At limits, input is ignored (no bounce). Animations are skipped under
`prefers-reduced-motion`. Viewport memory is per page (§9.3). Browser zoom is independent.

### 2.3 Smart zoom (`Shift+2`)
Selection bounds include stroke and effects. Fit target uses **64 px** padding, zoom clamped `[0.25, 2]`.

| Condition | Result |
|---|---|
| No selection | Same as `Shift+1` |
| Selection wider/taller than viewport | Fit, `zoom ≤ 2` |
| Selection < 10% of viewport | 200% or fit, whichever is smaller |
| Current zoom within ±8% of target | Center only, do not zoom |
| Editing `TEXT` | Fit width only, keep 12 px caret margin, `zoom ≤ 2` |

Transition 160 ms; instant under reduced motion. `Shift+Enter` centers without zoom and is what
"scroll to selection" uses.

### 2.4 Rulers and guides
Rulers toggle `Shift+R`, are **24 px** thick, and draw in screen space. Ticks choose `1/2/5 × 10^n` so
labels are ≥ 64 px apart; minor ticks disappear below 8 px spacing. Selection range is a 2 px accent
band; pointer is a 1 px marker.

Drag from a ruler to create a guide; drag it back to delete. `Alt`-drag duplicates. Guides are 1 px at
60% accent (2 px when hovered/dragged), snap to edges/centers and integer px. Max **100** per page;
persist per user in `editorState.byUser[user].guidesByPage` **[proposed]**. Not exported, not shared.

### 2.5 Layout grids, pixel grid
`LayoutGrid` was referenced in 02 but undefined; this spec proposes:

```ts
interface LayoutGrid {                     // [proposed]
  id: string; type: 'COLUMNS' | 'ROWS' | 'GRID';
  visible: boolean; color: RGBA; alignment: 'MIN' | 'CENTER' | 'MAX' | 'STRETCH';
  count: number;                           // COLUMNS 1..24, ROWS 1..100
  gutter: number; margin: number;          // 0..200, 0..400; 2*margin < frame size
  sectionSize?: number; offset?: number;   // GRID 1..200, offset 0..10000
}
```
Defaults: desktop 12 columns / 24 gutter / 24 margin; mobile 4 / 16 / 16; grid 8 px. Max 8 grids per
node. Grids render above fills and below children (or as an overlay), clipped to the node, never
exported. GRID styles bind via `styleRefs` kind `GRID`. Snap is off unless `snapToLayoutGrids` is on
(4 px). Pixel grid shows only at `zoom ≥ 8` (1 px lines at 10%); toggle `Cmd/Ctrl+'`. Snap-to-pixel is
ON by default (`Cmd/Ctrl+Shift+'`), rounds committed x/y/w/h, and is bypassed while `Alt` is held.

### 2.6 Snapping and measurement
| Class | Threshold (screen px) | Candidates |
|---|---|---|
| Parent | 6 | Parent left/center/right, top/middle/bottom |
| Sibling | 6 | Visible, unlocked sibling edges/centers |
| Equal spacing | 4 | Gaps matching an adjacent gap |
| Guide | 4 | Active-page guides |
| Layout grid | 4 | Column/row/grid lines when enabled |
| Pixel | At commit | Integer px when enabled |

Competition: modifier locks first (`Shift` axis, `Alt` center); then lowest screen distance; then type
penalty parent `0` < sibling `1` < guide `2` < grid `3`; then closest edge to pointer; then lexical
`targetId` for determinism. Max one snap per axis per frame. A snap holds until the pointer moves
**8 px** away. `Cmd/Ctrl` disables snapping except pixel snap. Rotated nodes use their world AABB;
rotation is not snapped.

Guides are 1 px accent lines extended 16 px past matched edges. Distance labels are 11 px with a 2 px
pill; at most 4 measurements (nearest sibling each axis, then selection size). Equal gaps are pink
`#F24E9F` and labeled. Hidden/locked nodes never snap; auto-layout children do not snap during reorder.
Overlays fade in 120 ms on release.

Measurement tool `M`: drag between snap points; shows `dx`, `dy`, distance, angle (0.1°). `Shift`
constrains 0/45/90°; `Alt` measures centers. `Escape`, tool change, or page switch clears it. No
command; agents use node bounds.

### 2.7 Rendering, interaction, and large files
| Layer | Contents | `listening` |
|---|---|---|
| `bgLayer` | Page background, layout grids | false |
| `staticLayer` | Nodes not being dragged; cached frames | true (hit graph cached) |
| `dynamicLayer` | Dragged subtree, drop previews, ghost | false during drag |
| `overlayLayer` | Selection, handles, guides, measurements, marquee/lasso, insertion indicator | handles only |
| `presenceLayer` | Human/agent cursors, labels | false |

Rules: one `batchDraw()` per rAF, never inside `pointermove`; during drag only the dragged subtree,
overlays, cursor, and inspector X/Y/W/H (30 Hz) may re-render; auto layout re-solves only on insertion-
index change (≤ 30 Hz) and once on drop; hover hit tests are rAF-throttled (2 ms p95 budget). Cull by
`absoluteRenderBounds` expanded for effects against viewport + **256 px**. LOD: `zoom < 0.15` frames
with ≥ 20 children render as cached bitmaps and text as gray bars; effects skip below `0.4`; pixel grid
above 8×. Cache frames with ≥ 50 children when clean and `zoom < 1`; invalidate on descendant mutation,
theme change, or font load. 50,000 nodes/file and 2,000/frame are rejected at command time per 02 §18.1.

**Empty state:** no auto-created nodes. A 320×180 centered card offers Mobile/Tablet/Desktop/Watch
presets and "or press F"; dismiss on first node or `Escape`, per user per file.

**Huge-file mode** (current page > **10,000** nodes): force frame caching, skip effects below 50%,
disable pixel grid, hit-test containers first (leaf requires `Cmd/Ctrl` deep select), run search in a
worker, show a dismissible banner with the node count. Budgets in §13.3 still apply; two frames > 32 ms
within 2 s disable effects entirely and emit `perf.degrade`.

### 2.8 Touch and trackpad
One-finger drag pans (touch defaults to pan; Pencil/stylus draws with the active tool); two-finger
pans; pinch zooms; tap selects; long-press 500 ms opens context menu, then drag moves selection;
double-tap enters group or edits text; Pencil pressure is sampled for freeform; palm rejection ignores
touch 300 ms after Pencil input. Touch handles are 16 px with 44 px hit targets; hover outlines are
suppressed. MVP supports iPad Safari 16+ for select/transform/draw; full touch parity and radial menus
are deferred.

### 2.9 Agent-relevant affordances
- Viewport is readable as `{ pageId, pan, zoom }`; agents may set a preview viewport without changing
  the human's saved view.
- `screenToCanvas` / `canvasToScreen` are pure; `PresenceState.cursor` is always canvas space (02 §14).
- Zoom presets and frame presets are stable strings, so "focus selection" is reproducible.
- `SnapCandidate { axis, kind, targetId?, position, distancePx }` **[proposed]** is exposed read-only to
  explain placements; never persisted.
- Empty-state presets equal frame presets, so human- and agent-created frames are indistinguishable.

---
## 3. Selection & Direct Manipulation
### 3.1 Selection model
Selection is an ordered `NodeId[]` in `PresenceState.selection`, paint order (bottom to top), never
persisted in the document. Max **50,000** selected; transform commands cap at **2,000** nodes
(`SELECTION_TOO_LARGE`), bulk style edits cap at 2,000 with a progress toast above 500. Deepest-wins
normalization prevents ancestor + descendant duplicates. Locked nodes are not canvas-hit-testable
(02 I9); hidden nodes are not hit-testable at all. Both are selectable in the layer panel, and the
inspector shows a read-only banner with Unlock/Show.

### 3.2 Click, modifiers, hit testing
Hit shape: filled area for shapes; 8 screen px minimum band for `LINE`/`ARROW`; glyph box for `TEXT`;
frame fill for containers. **Tolerance 4 screen px** (8 px below 50% zoom). Nodes under 12×12 screen px
get a centered 12×12 hit target capped at the neighbor's center.

| Input | Result | Commands |
|---|---|---|
| Click | Topmost node; if inside a `GROUP`, select outermost non-entered group | none |
| `Shift`+click | Add/remove | none |
| `Cmd/Ctrl`+click | Deep select: leaf under pointer, bypassing groups/frames/instances | none |
| `Alt`+click | Cycle overlapping nodes at point | none |
| Double-click | Enter container and select child; `TEXT` enters editing | none |
| Second click on selected `GROUP` | If moved < 3 px, enter and select child; drag > 3 px moves the group | none |
| Empty canvas click | Clear selection | none |
| Right-click | Context menu; selects target only if outside current selection | none |

Group entry: first click selects the `GROUP` (dashed outline, union bounds); second click enters;
`Escape` exits one level. `FRAME`, `COMPONENT`, `COMPONENT_SET` behave similarly. `INSTANCE` children
are mirrored; deep select selects the child and the inspector says "edit as override" (04).

### 3.3 Marquee and lasso
Box select (`V` + drag on empty canvas) selects top-level nodes **fully contained** in the marquee;
containers select as a unit. Lasso (`Q` or flyout) samples every 4 screen px, simplifies at 2 px, and
selects a node when **≥ 50%** of its world AABB is inside the polygon. `Shift` adds, `Alt` subtracts;
`Escape` cancels and restores the prior selection. Auto-scroll within 24 px of an edge at 8 px/frame.
Visual: 1 px accent outline, 8% fill. Spatial-index query < 8 ms for 10k nodes; selection store updates
at 30 Hz, marquee visual at 60 Hz.

### 3.4 Outlines, handles, transform
| Element | Spec |
|---|---|
| Hover | 1 px accent at 40%, 100 ms delay; hidden during drag/edit/draw |
| Selection | 1.5 px accent; multi-select 1 px dashed AABB |
| Entered container | 1 px dashed accent at 60% + 2 px corner tick |
| Resize handles | 8×8 screen px, 4 px slop; 12×12 below 50% zoom, edge midpoints hidden |
| Rotation zone | 24 px band outside corners, corner-oriented cursor |
| Radius handle | 8 px circle at top-left when radius is uniform and `zoom ≥ 0.25` |
| Locked/protected | No handles, 1 px dotted gray outline, read-only inspector |

Rotation: center pivot (`Alt` = opposite corner), `Shift` snaps 15°, `Alt` 1°. Inspector accepts
`-360..360` degrees, stores radians. Multi-select rotates each node around the selection center; the box
is oriented only if all rotations match. `LINE` direction changes via endpoints, not rotation.

Resize modifiers: `Shift` locks corner aspect; `Alt` resizes from center; `Cmd/Ctrl` disables snapping;
`Space` pauses and repositions. Constraints apply only outside auto layout or to `ABSOLUTE` children.

| Constraint | Horizontal effect (vertical analogous) |
|---|---|
| `LEFT` / `TOP` | x/y fixed, size fixed |
| `RIGHT` / `BOTTOM` | x/y adjusts by Δsize, size fixed |
| `CENTER` | x/y adjusts by Δsize/2, size fixed |
| `LEFT_RIGHT` / `TOP_BOTTOM` | x/y fixed, size stretches by Δsize |
| `SCALE` | position and size scale proportionally |

Minimum **1×1**; `maxSize` enforced. `TEXT` resize changes the box, never font size. `IMAGE` follows
`scaleMode`; `Shift` preserves aspect. Frames may resize below content; overflow clips per
`clipsContent`.

### 3.5 Nudge, push, flip
Arrow = 1 px; `Shift` = 10; `Alt` = 0.1 (subpixel). `Cmd/Ctrl+Arrow` on an auto-layout child reorders
one slot (`node.reorder`) and does not change x/y. Arrow on a selected vector vertex moves it
(`vector.setVertex`).

**Push** (default ON): after an axis-dominant drag, an overlapped sibling moves by `overlap + 8 px` on
that axis. Only same-parent, unlocked, visible, non-auto-layout, non-`ABSOLUTE` siblings; one cascade
level, max 20 nodes. Arrow nudges never push; `Cmd/Ctrl` disables push. Move + pushes = one
`HistoryEvent` ("Move and push 3 layers").

Flip `Shift+H`/`Shift+V` toggles `scaleX`/`scaleY` (`1`/`-1`). Containers mirror children. `TEXT` is the
exception: transform stores the flip, but glyphs stay readable and only box origin/alignment mirror;
exporters must match.

### 3.6 Drag, reparenting, auto-layout insertion
Drag starts after **3 px** or 150 ms. `Alt`-drag duplicates first (new IDs, inserted above source).

Reparent rules: candidates are `FRAME`, `SECTION`, `COMPONENT`, `COMPONENT_SET`; `GROUP` is not a
canvas drop target, `INSTANCE` never is (children mirrored). Pick the topmost visible, unlocked
container under the pointer that is not the dragged node/descendant. Preserve world position by
inverting the new parent's matrix; cancel on a singular matrix. `Cmd/Ctrl` prevents reparenting and
keeps the current parent at the dropped world position. Hovering a page row for **500 ms** switches
pages and continues the drag; drop preserves page-space coordinates. Dropping into a descendant fails
`REPARENT_CYCLE`.

Auto-layout insertion: a 2 px accent line spanning the cross axis, computed from the pointer along the
primary axis. Child zones: leading 25% = before, trailing 25% = after, middle 50% = into a container
child or nearest gap. Drop commits `node.reparent` + `node.reorder` in one event; preview solves at
≤ 30 Hz, final once. `Cmd/Ctrl` switches to **absolute**: `layoutPositioning: 'ABSOLUTE'`, dashed 1 px
rectangle + "Absolute" badge (`layout.setChildLayout`). Dragging out clears `layoutChild` props and
sets world x/y; dragging within the same container reorders only.

### 3.7 Clipboard, duplicate, delete, lock, rename
| Action | Shortcut | Behavior | Commands |
|---|---|---|---|
| Copy | `Cmd/Ctrl+C` | `ClipboardDocument`; max 10k nodes / 5 MB, else offer copy-as-PNG | none |
| Cut | `Cmd/Ctrl+X` | Copy then delete; skips hidden/locked with a warning | `node.delete` |
| Paste | `Cmd/Ctrl+V` | Remap IDs; roots at pointer (or in place); cascades +10 px | `node.create` batch |
| Paste in place | `Cmd/Ctrl+Shift+V` | Exact original page coordinates | `node.create` |
| Paste over selection | `Cmd/Ctrl+Alt+Shift+V` | One selected node; uniform scale 0.25×–4× to its bbox; replace | `node.delete`, `node.create` |
| Duplicate | `Cmd/Ctrl+D` | Insert above source; +10 px free canvas, no offset in auto layout | `node.duplicate` |
| Delete | `Delete`/`Backspace` | Delete selection; component with instances asks confirmation | `node.delete` |
| Lock / Hide | `Cmd/Ctrl+Shift+L` / `H` | Toggle `locked` / `visible`; protected nodes cannot unlock | `node.setLocked`, `node.setVisible` |
| Rename | `Cmd/Ctrl+R` or double-click | 1–255 chars; empty resets to type default | `node.rename` |

```ts
interface ClipboardDocument {                // [proposed]
  schema: 'kanvas.clipboard/1'; sourceFileId: FileId; copiedAt: number;
  rootIds: NodeId[]; nodes: Record<NodeId, Node>;
  assetRefs: AssetId[]; variableKeys: string[]; styleKeys: string[]; componentKeys: string[];
}
```
Cross-file paste resolves variables/styles/components by stable `key`; unresolved links detach to last
values and warn. Dangling `INSTANCE` masters follow 02 §11.5. Pasting into text inserts plain text.

### 3.8 Undo, redo, coalescing
Every `HistoryEvent` is one undo step (02 I10); `history.undo`/`history.redo` are engine operations,
not commands. Local stack depth **200**; server retains the full log (07).

| Gesture | `coalesceKey` | Window |
|---|---|---|
| Move/resize/rotate | `drag:<nodeId>:transform` | Whole gesture; commit on pointerup |
| Inspector scrub | `scrub:<nodeId>:<path>` | 600 ms idle or blur |
| Text typing | `text:<nodeId>:characters` | 750 ms idle or blur/Escape |
| Nudge | `nudge:<nodeId>:transform` | 400 ms idle, same axis/direction |
| Rename | `rename:<nodeId>:name` | Commit, max 3 s |
| Color picker | `paint:<nodeId>:<path>` | Close or 500 ms idle |
| Numeric field | `field:<nodeId>:<path>` | 800 ms idle or Enter |

Different key = new event; multi-node drags use the primary ID + selection hash. Undo restores
selection where possible; redo clears on a new command. Collaboration rebasing is in 07.

### 3.9 Cursors and agent affordances
Cursors: arrow (move), crosshair (draw/text/pen/measure), I-beam over text, grab/grabbing (pan), eight
resize cursors, rotate glyph, comment/measure custom cursors, `not-allowed` on locked drag, `no-drop`
on cycle. 2× assets, hotspot `2 2`.

Agent: selection is an ordered ID list, never z-index; deep-select is deterministic; `InsertionTarget {
parentId, index, mode: 'AUTO'|'ABSOLUTE', edge }` **[proposed]** is both preview and `node.reparent`
input; `preserveWorldTransform: true` makes "move into frame" reproducible. Agent runs SHOULD reuse
`coalesceKey: "agent:<runId>:<stepId>"` so one step is one reviewable event. Locked/protected errors
match human errors.

---
## 4. Shape Tools & Drawing
### 4.1 Tool tray
Floating toolbar, bottom-center, 48 px high, ≤ 720 px wide, 32×32 buttons, 4 px gaps. Pressing the
active tool's key returns to Move. `Escape` cancels a draw. Shortcuts are disabled in text fields/IME.

| Tool | Key | Creates | Default | Notes |
|---|---|---|---|---|
| Move / Hand | `V` / `H` | — | — | Select/transform / pan |
| Frame | `F` | `FRAME` | 400×300 | Preset flyout |
| Slice | `S` | `SLICE` | 100×100 | Export region, no paint |
| Rectangle | `R` | `RECTANGLE` | 100×100, fill `217,217,217` | Rounded via flyout (radius 12) |
| Ellipse | `O` | `ELLIPSE` | 100×100 | `Shift` = circle |
| Line / Arrow | `L` / `Shift+L` | `LINE` / `ARROW` | 100×1 | Stroke-only; arrow `arrowEnd: true` |
| Polygon | `Shift+3` | `POLYGON` | 100×100, `pointCount: 3` | Triangle |
| Diamond | `Shift+4` | `POLYGON` | `pointCount: 4`, rotation 45° | Rotated polygon, not a type |
| Star | `Shift+5` | `STAR` | 100×100, 5 points, `innerRadius: 0.38` | |
| Pen / Pencil | `P` / `Shift+P` | `VECTOR` | — | §5 |
| Text | `T` | `TEXT` | auto-width | §6 |
| Comment / Measure | `C` / `M` | thread / — | — | 04 / §2.6 |

### 4.2 Draw-drag behavior
Click creates at default size centered on click; drag creates from anchor to pointer with a live
preview. `Shift` constrains square/circle/45°/regular polygon; `Alt` draws from center; holding `Space`
moves the in-progress shape; `Escape` cancels with no history. Drag < 3 px falls back to default size.
On pointerup emit one `node.create`, select the node, keep the tool active unless `toolLock` is off.
Drawing inside a `FRAME` parents to that frame (same rules as §3.6); `Cmd/Ctrl` on pointerup keeps it at
page root. Inside auto layout, insert at the computed index with `layoutPositioning: 'AUTO'`.

New nodes: `opacity: 1`, `blendMode: 'NORMAL'`, `visible: true`, `locked: false`, no stroke/effects,
`sizing` `FIXED`/`FIXED`, metadata from actor. `FRAME`: `clipsContent: true`, white fill. `GROUP`: no
fill/clip. Text fill `30,30,30`.

### 4.3 Per-shape editing
| Shape | On-canvas | Inspector |
|---|---|---|
| `RECTANGLE` | Uniform radius handle; double-click → vector edit | Radius per corner, corner smoothing 0–1 |
| `ELLIPSE` | Arc handles when `arcData` exists | Start/end 0–360°, inner radius 0–1, reset |
| `POLYGON` | Point-count updates around center | `pointCount` 3–12 |
| `STAR` | Inner-radius handle on first inner vertex | `pointCount` 3–12, `innerRadius` 0–1 |
| `LINE`/`ARROW` | Endpoint drag; `Shift` 15°; `Alt` moves both | Arrow start/end, stroke |
| `SLICE` | Resize only; 1 px dashed cyan | Export settings (§4.5) |

Shape mutations: `node.setCornerRadius`, `node.setCornerSmoothing`, `node.setArc`, `node.setPointCount`,
`node.setInnerRadius`, `node.setPoints`. Double-click on a shape converts it to editable `VECTOR` (one
history event; lossy only for corner smoothing); `TEXT` edits, `FRAME`/`GROUP` enter.

### 4.4 Frames
| Preset | Size | Name |
|---|---|---|
| Mobile | 390×844 | `Mobile` |
| Tablet | 768×1024 | `Tablet` |
| Desktop | 1440×900 | `Desktop` |
| Watch | 396×484 | `Watch` |
| Presentation | 1920×1080 | `Slide` |
| Custom | last used | `Frame` |

`F` then drag/click, or preset flyout. Double-click a preset creates at viewport center and fits.
Nesting max **32** (`MAX_NESTING_DEPTH`). Tool-created frames clip; frame-from-selection does not.
Toggle clip with `Cmd/Ctrl+Alt+C`. Frame background is `fills`; `backgroundOpacityOverride` affects
display only. Label 12 px above top-left, hidden below 30% zoom unless selected; click selects,
double-click renames.

**Frame from selection** `Cmd/Ctrl+Alt+G`: `FRAME` at selection world AABB, 0 padding, no clipping,
reparents selection preserving world transforms; different parents → page root; a single frame gets a
parent wrapper (`node.wrapInFrame`). **Group** `Cmd/Ctrl+G`; **ungroup** `Cmd/Ctrl+Shift+G`; both
preserve world transforms.

### 4.5 Slices
`SLICE` has no paint, is click-through (select via marquee or layer panel), and does not affect layout,
group bounds, or auto layout. Visual: 1 px dashed `#0D99FF` + 8 px label. Create with `S` or "Create
slice from selection"; default PNG 1×. Allowed under page root, `FRAME`, `SECTION`; not under
`INSTANCE`/`COMPONENT_SET`. Max **200** slices/page, **50** export settings each. Excluded from frame
thumbnails and frame-from-selection bounds. Export mechanics: 08.

### 4.6 Style defaults and agent affordances
With no selection, the inspector shows **Defaults for [active tool]**: fill, stroke, effects, radius,
text style, frame background/clip. Edits affect only the next node and persist locally in
`ToolDefaults` **[proposed]**, not in the file. "Use style from selection" samples the first selected
node (disabled for multi-select, `GROUP`, `SLICE`); "Reset defaults" restores §4.2 values; "Apply to all
tools" copies shared properties.

Agent: defaults are preferences, so agents must pass explicit properties; frame presets match
`DesignIntent.target.framePreset` (02 §15.2); `node.wrapInFrame`/`node.group` return new IDs;
`node.create` is idempotent under retry when the agent supplies IDs (`NODE_ID_CONFLICT` on duplicates).

---
## 5. Vector & Pen Tool
### 5.1 Pen state machine
`VECTOR` stores `paths: VectorPath[]` with cubic commands and optional `vertices` (02 §6.1). One path per
session unless joined.

| Input | Idle | Path open | Commands |
|---|---|---|---|
| Click | Start `M` | Add `L` corner vertex | `node.create` on finish |
| Drag | Start + mirrored handles | Add `C` smooth vertex | `node.create` on finish |
| Click first vertex ≤ 8 px | — | Close `Z`, finish | `node.create` |
| `Enter` / `Escape` | — | Finish open path / finish or discard if < 2 vertices | `node.create` or none |
| `Backspace` | — | Remove last vertex; cancel if none | none |
| `Shift` | Constrain 15° | Constrain segment 15° | none |
| `Alt` while dragging handle | Break symmetry (`DISCONNECTED`) | Same | none |
| Double-click path | — | Add vertex at nearest segment point | `vector.addVertex` |

After finish, select the `VECTOR`, keep pen active. Clicking an existing `VECTOR` with the pen enters
vertex editing. `Escape` exits vertex editing before clearing selection.

### 5.2 Vertex and path editing
Double-click a `VECTOR` to edit. Vertices are 7×7 screen px squares (filled when selected); handles are
5 px circles with 1 px lines. Click a segment to insert a vertex at the closest point (split `Q`/`C`
without changing curvature); `Delete` removes selected vertices (min 2 per open, 3 per closed path).
Drag moves; `Shift` constrains to segment direction; arrows nudge 1 px (`Shift` 10).

Corner types: `MIRRORED` (equal angles/lengths), `ASYMMETRIC` (equal angle, free lengths), `DISCONNECTED`
(free), `STRAIGHT` (no handles). `Cmd/Ctrl+Shift+B` breaks at a vertex; `Cmd/Ctrl+Shift+J` joins two open
endpoints ≤ 24 px (straight segment if only endpoints); `Cmd/Ctrl+Shift+O` closes a path. Fill rule is per
node (`NONZERO`/`EVENODD`). Open paths with `fill: true` are implicitly closed for fill but open for
stroke; inspector shows "Open · filled". Closed with `fill: false` is stroke-only.

### 5.3 Freeform (pencil)
`Shift+P`. Sample coalesced pointer events up to 120 Hz; discard points < 0.5 px apart. On release run
RDP simplification at **0.6 px** then curve fit with ≤ **4 px** error (02 §6.1), producing an editable
`VECTOR`. Simplify is deterministic and must not change visual bounds by more than 4 px. Stylus pressure
may modulate sampling/preview but the model has one `Stroke.weight`; variable-width strokes are deferred
(§14.2). Default 2 px round stroke. Clipping is by ancestor `clipsContent`, never destructive. Above
2,000 vertices, offer "Simplify further" at 1.5 px before finishing.

### 5.4 Booleans, flatten, outline stroke
Requires 2+ unlocked `VECTOR`, `RECTANGLE`, `ELLIPSE`, `POLYGON`, or `STAR` nodes. `TEXT`, `IMAGE`,
`FRAME`, `GROUP`, `INSTANCE`, `LINE` are rejected (`BOOLEAN_UNSUPPORTED_TYPE`); text outlining is an
export concern (08).

| Operation | Shortcut | Result |
|---|---|---|
| Union | `Cmd/Ctrl+Alt+U` | `BOOLEAN`, `operation: 'UNION'` |
| Subtract | `Cmd/Ctrl+Alt+S` | `BOOLEAN`, first child is base |
| Intersect | `Cmd/Ctrl+Alt+I` | `BOOLEAN` |
| Exclude | `Cmd/Ctrl+Alt+X` | `BOOLEAN` |

Browser-reserved combos (`Cmd+Alt+I/S`) may not fire; toolbar/context menu remains, and bindings are
rebindable. Result sits at the first child's z-position, children in paint order, name inherited or
`Union 1` etc. Children stay live; double-click enters; reorder changes the result; `operation` is
editable (`boolean.setOperation`). Fill/stroke apply to the composed result; child paints ignored except
nested booleans. Empty result keeps the node with an amber "Empty boolean result" badge. Flatten
`Cmd/Ctrl+Alt+E` → `vector.flatten`, irreversible except undo, blocked above 20k vertices
(`BOOLEAN_TOO_COMPLEX`). `vector.outlineStroke` is disabled for dashed, gradient, or zero-width strokes,
with a reason tooltip.

**Vector network limitation.** Multiple subpaths and holes are supported; arbitrary shared-vertex
networks (a vertex in three segments with independent handles) are not. SVG networks split into
subpaths and flag `NETWORK_SPLIT`; visual fidelity is kept, topology is not. Deferred (§14.2).

### 5.5 SVG import mapping
| SVG | Result | Rule |
|---|---|---|
| `rect` | `RECTANGLE` | `rx/ry` → radius / `Corners`; transforms preserved |
| `circle`/`ellipse` | `ELLIPSE` | Center/radii mapped |
| `line` | `LINE` | Stroke required |
| `polyline`/`polygon` | `VECTOR` | Open/closed vertices |
| `path` | `VECTOR` | `M/L/C/Q/Z` 1:1; arcs → cubics ≤ 0.1 px error |
| `g` | `GROUP` | Transform on group |
| `use`/`symbol` | `INSTANCE` or `VECTOR` | Unresolved → warning |
| `text`/`tspan` | `TEXT` + ranges | Unknown fonts enter §6.5 |
| `image` | `IMAGE` | SHA-256 dedupe; data URIs decoded |
| `clipPath` / `mask` | `FRAME` clip / `isMask` | 02 §5 |
| `filter` | effects best effort | Unsupported primitives dropped with warning |
| CSS/animation/script | ignored | Warning summary lists drops |

One `HistoryEvent` with `origin: 'IMPORT'`; roots go under the selected frame or page root. SVG also
stored as `Asset` kind `SVG`. Warning codes: `UNSUPPORTED_FILTER`, `NETWORK_SPLIT`, `MISSING_FONT`,
`MISSING_ASSET`.

### 5.6 Agent-relevant affordances
Each vertex has a stable `Vertex.id`; `vector.setVertex` produces small deltas, `vector.setPaths` is for
bulk edits. Boolean state is `{ operation, childIds }`; keep it live unless the user asks to flatten.
Empty results and import warnings are machine-readable on the command result. SVG import never silently
drops features.

---
## 6. Text
### 6.1 Creation and auto-resize
| Creation | Result |
|---|---|
| `T` + click | Auto width, caret at click, `textAutoResize: 'WIDTH_AND_HEIGHT'` |
| `T` + drag | Fixed box, `textAutoResize: 'NONE'`, wraps inside |
| Double-click `TEXT` | Edit; caret at click; double-click word selects it |
| Empty commit | Node deleted (undo restores); no zero-length text persists |

`textAutoResize` and `sizing` are written atomically. The `text` validator repairs mismatches.

| UI | `textAutoResize` | `sizing` | Behavior |
|---|---|---|---|
| Auto width | `WIDTH_AND_HEIGHT` | `HUG`/`HUG` | Width follows longest line, no wrap |
| Auto height | `HEIGHT` | `FIXED`/`HUG` | Fixed width, height grows |
| Fixed | `NONE` | `FIXED`/`FIXED` | Clips overflow + warning |
| Fill width (AL) | `HEIGHT` | `FILL`/`HUG` | Width from solver, height hugs |
| Truncate | `TRUNCATE` | `FIXED`/`FILL` + `FIXED` | `truncation: 'ENDING'`, optional `maxLines` |

`WIDTH_AND_HEIGHT` + `FILL` normalizes to `HEIGHT` + `FILL`. Corner resize changes the box, never
`fontSize`; `Shift` preserves box aspect. `maxLines` 1–1,000, only with `TRUNCATE`; ellipsis and
`TextDerived.overflowLines` drive the badge.

### 6.2 Editing model
Konva renders; a transparent DOM `<textarea>` overlay handles caret, selection, IME, and accessibility.
Caret 1 px accent, blink 530 ms on/off, paused while typing; selection accent at 30%; caret stays 1
screen px below 50% zoom. **IME:** `compositionstart` suspends writes, `compositionupdate` renders with a
1 px underline, `compositionend` commits once. Rich ranges use UTF-16 offsets and never overlap.

Paste rich text maps bold/italic/underline/strikethrough/links to `rangeStyles`; lists map when the whole
selection is a list; images/tables drop with a warning. Enter = paragraph, `Shift+Enter` = line break.
`Tab` commits and moves focus (accessibility); `Cmd/Ctrl+Tab` inserts a tab. `Escape` commits and selects
the node; `Cmd/Ctrl+Enter` commits and returns to Move. Click outside commits. Undo edits text first,
then normal history.

### 6.3 Typography controls
| Control | Range / values | Notes |
|---|---|---|
| Family / style | FontRegistry / Regular…Bold Italic | Searchable; missing flagged |
| Size | 1–1,024 px | ±1, `Shift` ±10 |
| Line height | `AUTO`, `PIXELS` 0–2,000, `PERCENT` 50–500% | `AUTO` from metrics |
| Letter spacing | `PIXELS` −100–1,000, `PERCENT` −50–500% | Negative allowed |
| Paragraph spacing / indent | 0–1,000 px | First line indent |
| Align H / V | `LEFT/CENTER/RIGHT/JUSTIFIED`; `TOP/CENTER/BOTTOM` | `Cmd/Ctrl+Shift+L/C/R/J` |
| Case | `ORIGINAL`, `UPPER`, `LOWER`, `TITLE`, `SMALL_CAPS`, `SMALL_CAPS_FORCED` | Visual only |
| Decoration | `NONE`, `UNDERLINE`, `STRIKETHROUGH` | `Cmd/Ctrl+U`, `Cmd/Ctrl+Shift+X` |
| Lists | `NONE`, `ORDERED`, `UNORDERED`, `TODO` | `Cmd/Ctrl+Shift+7/8/9`; spacing 0–200 |
| Color | `textStyle.fills` | Solid/gradient/image, style/token |
| Hyperlink | URL | `Cmd/Ctrl+K`; underline by default |

Multi-select shows **Mixed** for differing values. Setting a value applies to all selected text nodes;
range styling across a mixed selection asks "Apply to 4 text layers?" (confirm can be disabled).

### 6.4 Text in auto layout
Hug contributes intrinsic metrics; fill-width resolves width then grows height; truncation clamps height
and reports `overflowLines`. `counterAxisAlign: 'BASELINE'` uses `TextDerived.baselineOffsets`, only for
`HORIZONTAL`; non-text children align `MIN` and show an info note. Font/line-height changes re-run the
intrinsic pass, resizing HUG ancestors in the same frame with no intermediate event.

### 6.5 Fonts, loading, missing fonts
`FontState` **[proposed]**: `LOADING | READY | MISSING | ERROR`. Text renders immediately with fallback
metrics; input never blocks; reflow happens once on load. Failure sets `MISSING`, shows an amber badge
and file banner "3 missing fonts — 12 text layers", and opens a dialog with **Replace with fallback**
(`Inter`), **Upload font** (04), **Choose closest available**. Replace uses `font.replaceMissing` for all
affected nodes in one undoable batch with `{ oldFamily, oldStyle, newFamily, newStyle }`.

Fallback: requested → uploaded → system UI → Noto Sans (CJK per script). MVP bundles Inter, 50 curated
Google families, uploaded fonts; CJK renders with Noto SC/JP/KR and correct line breaking. **Arabic,
Hebrew, and full bidi are deferred**; RTL renders LTR with a warning and the direction control is
disabled.

### 6.6 Placeholder text, text components, accessibility
Styles never carry content. Placeholder content is a component property or a non-empty `TEXT` named
`Placeholder`; empty text is not persisted. `TEXT` named `label`, `title`, or `text` auto-exposes a `TEXT`
component property; instance edits are overrides (02 §11.4). Empty override falls back to the master
default with an "empty override" hint.

`TextA11y` **[proposed]**: `{ role: 'NONE'|'HEADING'|'PARAGRAPH'|'LABEL'|'CAPTION'; headingLevel?: 1..6 }`,
default `NONE`. Inspector "Semantics" row; heading level normalizes role; announcements read "Heading
level 2, 'Welcome back'". Export maps to `h1..h6`, `p`, `label`, `figcaption`, `span` (08). A11y audit
flags skipped levels/empty headings.

### 6.7 Agent-relevant affordances
`characters` is authoritative; use `text.setCharacters` and UTF-16 ranges. `TextDerived` exposes
measured size, line count, baselines, overflow; agents can detect truncation without rendering.
`fontState`/missing-font lists are machine-readable; never substitute fonts silently. `a11y` survives
copy/paste and overrides.

---
## 7. Auto Layout
### 7.1 Apply, infer, remove
`Shift+A` on a `FRAME`/`COMPONENT` calls `layout.setAutoLayout`; on a `GROUP` it wraps in a `FRAME`
(`node.wrapInFrame`), then applies layout and deletes the group; on multiple siblings it calls
`layout.wrapSelection`. `TEXT`, `VECTOR`, and `INSTANCE` are rejected (`AUTOLAYOUT_UNSUPPORTED_PARENT`);
instances inherit the master. Remove via inspector/context menu (`layout.removeAutoLayout`): children
keep last solved geometry and become `FIXED`, `ABSOLUTE` x/y is kept, one event.

Inference prevents a visual jump: direction = wider-than-tall → `HORIZONTAL` else `VERTICAL`; gap = median
adjacent gap rounded to 4 px, clamped 0–64 (one child = 12); padding = minimum child-to-edge distance
rounded to 4 px, per side, clamped 0–64; parent `FIXED` at current size for an existing frame, `HUG` for
a new wrapper; child cross-axis `FILL` when it spans ≥ 99% of inner cross-size (±1 px), else `FIXED`;
primary `FIXED`; align `MIN`/`MIN`; no wrap; no clip; strokes excluded.

### 7.2 Controls
```
Auto layout                          [−] [+]
Direction [→][↓]   Wrap [off]   Gap 12   Counter 12
Padding   [12][12][12][12] [link]
Align     [MIN ▾]  [MIN ▾]   Align content [MIN ▾]
Clip [x]   Include strokes [ ]   Sizing W [HUG] H [FIXED 320]
Min/Max W 120/480  H —/—   Child: [AUTO] Grow [0] Align [INHERIT]
```
| Control | Values | Field | Commands |
|---|---|---|---|
| Direction / wrap | `HORIZONTAL`/`VERTICAL`; `NO_WRAP`/`WRAP` | `direction`, `wrap` | `layout.setDirection`, `layout.setWrap` |
| Gap / counter | 0–1,000 px | `itemSpacing`, `counterSpacing` | `layout.setSpacing` |
| Padding | 0–1,000 per side, link toggle | `padding.*` | `layout.setPadding` |
| Primary / counter align | `MIN/CENTER/MAX/SPACE_BETWEEN`; `MIN/CENTER/MAX/BASELINE` | `primaryAxisAlign`, `counterAxisAlign` | `layout.setAlignment` |
| Align content | `MIN/CENTER/MAX/SPACE_BETWEEN` | `alignContent` (wrap only) | `layout.setAlignment` |
| Clip / strokes | boolean | `clipsContent` (mirrors `FRAME.clipsContent`), `strokesIncludedInLayout` | `node.setClipsContent`, `layout.setStrokesIncluded` |
| W/H sizing | `FIXED/HUG/FILL` | `sizing` | `node.setSizing` |
| Min/max | px or `—` | `minSize`, `maxSize` | `node.setMinMax` |
| Child positioning | `AUTO`/`ABSOLUTE` | `layoutChild.layoutPositioning` | `layout.setChildLayout` |
| Child grow/align | `0/1`; `INHERIT/MIN/CENTER/MAX/STRETCH` | `layoutChild.*` | `layout.setChildLayout` |

Normalization: primary `FILL` ⇔ `layoutGrow: 1`; cross `FILL` ⇔ `layoutAlign: 'STRETCH'`; both written
atomically, mismatch repaired with a warning. `layoutChild` min/max fields are projections of
`BaseNode.minSize`/`maxSize`, never persisted separately. `FILL` + `ABSOLUTE` is invalid
(`ABSOLUTE_FILL_CONFLICT`); min > max clamps with a warning.

### 7.3 Sizing and resize
FILL children split free space equally, clamp by min/max, redistribute once (02 §7.1). `HUG` parents wrap
content. `FILL` inside a `HUG` parent is treated as `HUG` and warns `FILL_IN_HUG` (prevents collapse).
`FIXED` + `WRAP` breaks lines greedily; line cross-size is the max child cross-size.

Dragging a handle on a `HUG` axis silently converts it to `FIXED` ("Resize frame"); the other axis stays
`HUG`. Double-clicking the handle resets `HUG`. A `FILL` parent cannot be resized directly (tooltip:
"Width is set by the parent"). Resizing smaller than content with no shrinkable child reports overflow.
Visible clipping follows `clipsContent`.

### 7.4 Order, absolute children, nesting
Layer order equals primary order (first = left/top). Canvas/panel drag emits `node.reorder` and the
solver repositions in the same transaction; `Cmd/Ctrl+Arrow` moves one slot. `ABSOLUTE` children are
skipped by the solver, keep stored x/y, participate in z-order/clipping, and use constraints on parent
resize. `ABSOLUTE` → `AUTO` appends at the end and discards stored x/y after confirmation if it differs
by > 1 px. Nesting max **32**; reparent into self/descendant fails `LAYOUT_CYCLE`; `COMPONENT_SET` is
not a legal auto-layout child (`UNSUPPORTED_CHILD_TYPE`).

### 7.5 Distribution, overflow, precedence
`SPACE_BETWEEN` needs ≥ 2 visible `AUTO` children; 0–1 renders `MIN` with a note. Negative free space
sets spacing 0 and reports overflow. `BASELINE` is horizontal-only; non-text children align bottom, and
no text child falls back to `MIN`. `WRAP` + primary `HUG` is invalid → force `NO_WRAP` + warning. RTL
layout is not supported.

| State | Visual | Inspector |
|---|---|---|
| Child exceeds max | 16 px amber triangle top-right | "1 layer exceeds its maximum size" + Fix |
| Negative free space | Badge + 1 px dashed inner outline | "Content overflows by 24 px" |
| Clipped / not clipped | Badge remains | "Clipped" / "Visible outside frame" |
| Truncated text | Amber text badge | "3 lines hidden" |

Badge is 16×16, `aria-label="Layout overflow"`, clicking selects the first overflow node. A container
never silently grows unless its axis is `HUG`. Agents receive the same list.

Precedence: auto-layout `AUTO` child → solver (constraints inactive, inspector "Set by auto layout");
`ABSOLUTE` child → constraints; non-auto-layout parent → constraints; `INSTANCE` → master layout (04).
Removing auto layout reactivates preserved constraints using last solved geometry.

### 7.6 Agent-relevant affordances
`layout.setAutoLayout` requires the full `AutoLayout` object (no partial half-applied state) and returns
the solved `LayoutResult`. `solveLayout(root, doc, opts)` is a pure dry-run (02 §7.1). Overflow,
`FILL_IN_HUG`, wrap fallback, and baseline fallback are stable warning codes. Child order is explicit,
never inferred from x/y. `layout.wrapSelection` returns the new frame ID and inferred values.

---
## 8. Layer Panel & Navigation
### 8.1 Structure and virtualization
```
Pages                    [+]      Layers              [search…]
  Login                  ⋯       ▾ Login / Mobile   390×844
  Components             ⋯         ▾ Card           342×268
                                    T  Title
                                    ◇  Button/Primary
```
Width 240 px (200–360, persisted); page list max 240 px then scrolls. Rows **28 px**, 12 px indent,
16 px icons, 8 px right padding, 24 visible indent levels then horizontal scroll. Display is reverse
paint order (topmost first, 02 I2). Collapse state is session-local per page. Nodes with > 500 children
show "Show 500 more" in 100-row chunks. Virtualize visible + **8** overscan rows; `scrollTop / 28` maps
to index; 50k rows must scroll at 60 fps.

### 8.2 Drag, search, indicators
| Drop zone | Result |
|---|---|
| Top 25% / bottom 25% | Before / after row |
| Middle 50% container | Into container |
| Middle 50% collapsed container, 300 ms | Expand, re-evaluate |
| Panel edge ≤ 24 px | Auto-scroll 8 px/frame |
| `Cmd/Ctrl` held | Reorder only within current parent |
| Self/descendant | `no-drop`, red indicator, no commit |

Indicator: 2 px accent line inset 8 px; "into" uses a 2 px rounded outline. Drag starts at 4 px. Cross-
frame drops emit `node.reparent` + `node.reorder` in one event (world position not preserved for panel
drags). Multi-row drag preserves block order, grouped by parent, one batch. Canvas→panel drag is
deferred.

Search (`Cmd/Ctrl+F`): fuzzy name + filters `type:FRAME,TEXT`, `name:hero*`, `token:color/bg`,
`style:Heading/H2`, `component:Button/Primary`, `is:instance,hidden,locked,overflow`,
`has:autolayout,fill,effects,text`, `text:"sign in"`, `page:Login`. Debounce **120 ms**, max **200**
results, current page by default with an "All pages" toggle. `Enter` selects first, `Shift+Enter` adds,
`Cmd/Ctrl+A` selects all matches (cap 5,000). Empty state "No layers match" + Clear filters; invalid
filters show inline errors. p95 **50 ms**; worker above 10k nodes.

### 8.3 Toggles, indicators, navigation
Eye/lock 16 px, hover-revealed, `Alt`+click eye solos siblings then restores. Icons: purple diamond
master, hollow diamond instance, `state=hover` variant badge, amber overflow triangle, 6 px token dot,
mask glyph. Protected nodes show a shield (02 §5).

Breadcrumb above canvas: `Page / Frame / Group / Text`, 24 px segments, truncates from the left at 60%
width; clicking selects the ancestor. Frame quick-nav `Cmd/Ctrl+J` lists ≤ 500 frames, searchable,
centers at current zoom. Page list: click switches, double-click renames, drag reorders, context menu
duplicates/deletes.

Panel and canvas share `PresenceState.selection`; panel selection is optimistic and reconciles on echo.
Canvas selection scrolls the first row into view (`block: 'nearest'`, 8 px padding, 100 ms throttle)
without stealing focus. Off-screen selected nodes show a 24 px edge indicator with name; clicking
centers. Collapsing a selected node's parent keeps selection; breadcrumb still shows the path.

### 8.4 Keyboard and agent affordances
`Up/Down` move focus; `Left/Right` collapse/expand or move parent/child; `Enter` selects (`Cmd/Ctrl+Enter`
adds); `F2` renames; `Cmd/Ctrl+Shift+L/H` locks/hides; `Cmd/Ctrl+F` searches; `Escape` clears search then
returns focus to canvas.

Agents use `children[]` and `queryNodes` (02 §16), never scrape rows. Reorder accepts explicit
`parentId` + `index`. `CompactNode.flags` exposes hidden/overflow/instance state. Search IDs are stable;
agents must not depend on human scroll or selection-to-panel scrolling.

---
## 9. Pages
### 9.1 Page operations
| Action | Controls | Rules | Commands |
|---|---|---|---|
| Create | `+` or `Cmd/Ctrl+Alt+N` | Defaults `Page N`; opens immediately | `page.create` |
| Rename | Double-click / `F2` | 1–80 chars, trimmed; empty → `Page N`; duplicates warn | `page.rename` |
| Duplicate | Context menu | New node IDs; preserves background/viewport; rejects above 50k nodes | `page.duplicate` |
| Reorder | Drag, 2 px indicator | Optimistic, commit on drop | `page.reorder` |
| Delete | Context menu | Confirm with name + node count; cannot delete last page; 10 s undo, then version history only | `page.delete` |
| Background | Inspector page section | `SOLID` or `NONE`; default `247,247,248,1` | `page.setBackground` |

Max **500** pages/file (operational limit); Create disables with a toast beyond it. The current page has
a 2 px accent bar and bold name. Duplicate names get a warning dot.

### 9.2 Moving nodes across pages
Cut/paste creates new IDs; drag over a page row for **500 ms** switches pages and continues the drag;
"Move to page" search picker keeps selection. `node.moveToPage` keeps IDs (file-unique), updates
`pageId` on all descendants, preserves the parent if it moved too, otherwise re-parents to page root at
the same page-space position. Page switching loads lazily (02 §17).

### 9.3 Viewport memory and loading
02 §13 stores one viewport per user; this spec extends it **[proposed]**:

```ts
interface PageViewport { pan: Vec2; zoom: number; savedAt: number }

interface EditorUserState {                 // extends FileMeta.editorState.byUser[userId]
  pageId: PageId; zoom: number; viewport: Vec2; selection: NodeId[];
  pageViewports?: Record<PageId, PageViewport>;
  guidesByPage?: Record<PageId, Guide[]>;
}
```
Save before switching, restore with a 120 ms animation (instant under reduced motion). No entry → fit
content with 64 px padding; empty page → 100% at `(0,0)`. LRU prune beyond 100 pages.

Loading: skeleton + indeterminate bar after **200 ms**; p95 cold switch **1.2 s**, cached < 100 ms. LRU
keeps **5** pages; prefetch adjacent after 150 ms hover. Editing is disabled while loading; a drag that
triggers a page switch is cancelled. Failure keeps the previous page and shows a Retry toast. Deep links
`?page=pg_...&node=n_...` load, select, and focus; missing node shows "Layer no longer exists".

Agent: all commands include `pageId`; `queryNodes({ pageId })` loads transparently and returns a
`loading` warning if not resident. `node.moveToPage` is atomic and keeps IDs.

---
## 10. Inspect / Properties Panel
Panel width 248 px (200–360, persisted); header = editable name, type badge, size/child summary, `⋯`
menu (copy ID, select similar, reveal, docs). Sections collapse and remember state per user; only
relevant sections render. Empty selection shows Page + tool defaults (§4.6).

| Section | Controls | Commands |
|---|---|---|
| Align | 6 align buttons, distribute H/V, tidy-up | `node.setTransform` batch |
| Position & size | X/Y/W/H, rotation, flip H/V, constraints | `node.setTransform`, `node.setConstraints` |
| Auto layout | §7.2 | `layout.*`, `node.setSizing` |
| Appearance | Opacity 0–100, 17 blend modes, visible, locked | `node.setOpacity`, `node.setBlendMode`, `node.setVisible`, `node.setLocked` |
| Fill | Up to 12 paints: solid/gradient/image, opacity, blend, style, token | `node.setFill`, `style.apply`, `variable.bind` |
| Stroke | Weight 0–200, align, cap, join, dash, miter, paint, token | `node.setStroke` |
| Effects | Up to 8 shadows/blurs, reorder, per-effect visible | `node.setEffects` |
| Radius | Uniform/per-corner, smoothing 0–1 | `node.setCornerRadius`, `node.setCornerSmoothing` |
| Typography | §6.3 | `text.setStyle`, `text.setRangeStyle` |
| Export | PNG/SVG/JPG/WEBP/PDF, scale 0.5–4, suffix, preview | `node.setExportSettings` |
| Prototype / Component | Trigger/action/transition; variant/props/overrides/detach | 04 |
| Tokens | Bound variable pills per property, detach/swap/create | `variable.bind`, `variable.unbind`, `variable.create` |
| Description & annotations | `metadata.description` 0–2,000 chars; annotation list/resolve | `node.setDescription` |
| A11y (`TEXT`) | Role and heading level | `node.setA11y` |

**Multi-select.** Shared values render normally; differing values show italic **Mixed**. Numeric edits
set absolute values; X/Y are deltas from the selection AABB; `Alt+Enter` in W/H scales proportionally.
Unsupported nodes are skipped ("Applied to 3 of 4 layers"). Align disabled for one node. Mixed auto
layout applies the same shape to all supported frames. "Apply to selection" appears on hover for
style/token edits; `Alt`+click a section label resets it (confirm above 50 nodes).

**Token-bound properties.** A pill shows swatch + token name + link glyph; read-only per 02 §9.1; hover
shows resolved value for the active theme. Detach freezes the resolved value (`variable.unbind`). Editing
follows `tokenEditBehavior`: `CREATE_VARIABLE` (default), `DETACH`, or `ASK`. Picker states: grouped by
collection; name search (path segments bold); scope filter; value search (`#0D99FF`, `24`, `true`);
empty collection; no match; incompatible type disabled with "Expected COLOR, found FLOAT"; remote
library badge. Inline create asks name/collection/type (prefilled), defaults scopes to the property.

| Numeric input | Behavior |
|---|---|
| Drag label | Scrub 1 unit / 2 px; `Shift` ×10; `Alt` ×0.1; pointer capture |
| Arrows / wheel | ±1; `Shift` ±10; `Alt` ±0.1 |
| Typing | Decimals, sign, arithmetic `+ - * / ( )` (`(390-24)/2`); units `px`, `%`, `deg`, `×` |
| Commit / cancel | Enter or blur; `Escape` reverts; invalid shows red ring 1.2 s and never writes NaN |
| Clamp / precision | Property min/max with hint; display 2 decimals, store 3; pixel snap on commit |
| Disabled | Auto-layout-owned fields show a lock and remain focusable for screen readers |

Agent: property paths match `boundVariables` keys exactly; mixed reads return `{ mixed: true, values }`,
never a sentinel; commands are property-scoped for reviewable edits; token errors use `TOKEN_TYPE_MISMATCH`,
`TOKEN_SCOPE_INVALID`, `TOKEN_CYCLE`.

---
## 11. Search & Command Palette
`Cmd/Ctrl+K` unified palette (actions/layers/pages/components); `Cmd/Ctrl+F` layer search; `Cmd/Ctrl+Shift+F`
components; `Cmd/Ctrl+J` frame nav; `Cmd/Ctrl+P` command mode. Prefixes: `>` actions, `@` components, `#`
pages/frames, `~` layers; §8.2 filters work inline.

Ranking: exact > prefix > word-start > subsequence, then recency (30 days), then same page/frame, then
paint order, then ID. Show **20** results grouped; `Up/Down`, `Enter` run, `Cmd/Ctrl+Enter` run without
closing, `Escape` close. Actions show shortcuts and disabled reasons. Empty query shows 8 recent commands
and 5 examples. Debounce 80 ms, p95 50 ms, worker above 10k nodes.

Results: action runs a command; layer selects/reveals and centers if off-screen (`Cmd/Ctrl+Enter` adds);
page switches; frame switches page/selects/centers; component inserts an instance at viewport center
(`Cmd/Ctrl+Enter` reveals master). `Select similar` (`Cmd/Ctrl+Alt+A`): criteria type, component, style/text
style, size ±1 px, fill token, name glob; scope frame/page/file; live count; applies selection only, cap
5,000; remembers criteria.

Agent: palette actions map 1:1 to commands or view intents; results expose `{ kind, id, label, pageId,
path }`; select-similar criteria are serializable selectors; disabled actions return typed reasons.

---
## 12. Keyboard & Accessibility
| Category | Shortcuts |
|---|---|
| Tools | `V H F R O L Shift+L P Shift+P T S C M`; `Shift+3/4/5` polygon/diamond/star |
| Edit | `Cmd/Ctrl+Z` undo, `+Shift+Z` redo; `C/X/V`; `Shift+V` paste in place; `Alt+Shift+V` paste over; `D` duplicate; `Delete`; `G` group, `Shift+G` ungroup; `Alt+G` frame selection; `Shift+A` auto layout; `Shift+L/H` lock/hide; `R` rename |
| Arrange | `Cmd/Ctrl+]` forward, `[` backward; `Shift+]` front, `Shift+[` back; `Shift+H/V` flip |
| View | `Shift+1` fit, `Shift+2` focus, `Shift+0` 100%; `Shift+R` rulers, `Shift+G` grids, `Cmd/Ctrl+'` pixel grid; `Space` pan, `Cmd/Ctrl+wheel` zoom |
| Panels | `Cmd/Ctrl+K/F/J`; `F6`/`Shift+F6` next/previous region |
| Text | `Cmd/Ctrl+B/I/U`; `Shift+7/8/9` lists; `Shift+L/C/R/J` align |
| Boolean | `Cmd/Ctrl+Alt+U/S/I/X` union/subtract/intersect/exclude |

Shortcuts are suppressed in inputs/IME except `Escape`, `Cmd/Ctrl+Enter`, and text editing keys.

Focus order: top bar → left panel (Pages, Layers) → canvas → inspector → toolbar. `F6` cycles regions.
The canvas is one focusable region (nodes are not 50k tab stops); keyboard-only editing selects via the
layer panel/search, nudges with arrows, resizes with `Alt+Arrow`, rotates with `[`/`]` (1°, `Shift` 15°),
enters with `Enter`, exits with `Escape`.

`Escape` priority: close palette/menu/dialog → cancel drag/draw/marquee → commit/exit text → exit vector
edit → exit one entered group → clear selection → close transient overlays. It never closes the file.
Canvas uses `role="application"` with an instructions description; selection announces politely,
throttled **300 ms** ("Selected Frame 'Login / Mobile', 390 by 844"; "3 layers selected…"; "Selection
cleared"). Overflow announces once. Errors/destructive confirms are assertive and move focus. Layer tree
uses `role="tree"`/`treeitem` with `aria-level/expanded/selected/setsize/posinset`. Toolbar exposes
`aria-pressed` and `aria-keyshortcuts`. Zoom readout is a labeled spinbutton.

Reduced motion: instant pan/zoom, cursor playback uses `reducedMotionFallback` (02 §15.1), result pulses
become a static 2 px outline for 1.5 s. Focus ring 2 px accent + 2 px offset; targets ≥ 24×24; contrast
≥ 4.5:1 text, ≥ 3:1 handles; color never the only signal. At 200% browser zoom the inspector becomes an
overlay drawer below 1,024 px and the toolbar scrolls.

Agent: focus never determines target; announcements derive from the same selection summary agents read;
`aria-keyshortcuts` matches this table; agents must not assume cursor animation played.

---
## 13. Acceptance Criteria, Edge Cases, Performance
### 13.1 Acceptance criteria (Given / When / Then)
| ID | Given | When | Then |
|---|---|---|---|
| AC-2.1 | A 390×844 frame at 100% | Pinch-zoom on a corner | The canvas point under the gesture midpoint stays within 0.5 px; zoom clamps at 2–6400% |
| AC-2.2 | A selected 200×100 card off-screen | `Shift+2` | Card fits with 64 px padding, zoom 0.25–2, 160 ms (instant with reduced motion) |
| AC-2.3 | A node dragged near a sibling edge | Within 6 screen px | Edge snaps, one guide shows, commit is 1 px-aligned; `Cmd/Ctrl` suppresses it |
| AC-2.4 | A page with 12k nodes at 30% zoom | Pan continuously for 3 s | ≥ 55 fps, no full static re-render, effects skipped, banner visible |
| AC-3.1 | Overlapping group and child | Click, then click again without moving | First selects group; second enters and selects child; a > 3 px drag moves the group |
| AC-3.2 | Two frames side by side | Drag a child into the other frame and drop | `node.reparent` preserves world position; one history event; selection stays |
| AC-3.3 | An auto-layout frame with 3 children | Drag a new node over the gap between child 1 and 2 | 2 px insertion indicator at index 1; drop inserts there; one event |
| AC-3.4 | A node being dragged for 2 s | Release | Exactly one undo step; inspector updated at ≤ 30 Hz during drag |
| AC-4.1 | Nothing selected | Press `R`, drag 120×80 | `RECTANGLE` 120×80, default fill, selected, one event; `Shift` makes it square |
| AC-4.2 | Two overlapping shapes selected | `Cmd/Ctrl+Alt+U` | One live `BOOLEAN` union with both children; flatten yields a `VECTOR` |
| AC-5.1 | Pen with 3 clicks | Click the first vertex within 8 px | Closed `VECTOR`; fill applies; 3 editable vertices |
| AC-5.2 | A freeform stroke of 2,000 samples | Release | ≤ 2,000 simplified vertices, visual bounds error ≤ 4 px, editable path |
| AC-6.1 | A `TEXT` node at auto width | Drag its side handle | Width becomes fixed (`textAutoResize: HEIGHT` or `NONE`); font size unchanged |
| AC-6.2 | Two fonts, one unavailable | Reload the file | Text renders with fallback, banner lists 3 fonts/layers, input remains possible |
| AC-6.3 | A heading text node | Set `role: HEADING`, level 2 | Inspector persists `a11y`; announcement says "Heading level 2"; copy/paste preserves it |
| AC-7.1 | Two siblings selected | `Shift+A` | New `FRAME` wraps them, gap/padding inferred to 4 px, one event, returns frame ID |
| AC-7.2 | A fixed 320 px auto-layout frame with 3 `FILL` children | Resize to 260 px | Children redistribute equally; min/max clamps; overflow list empty or badge shown |
| AC-7.3 | A `HUG` parent with a `FILL` child | Apply layout | Child is treated as `HUG`, `FILL_IN_HUG` warning returned; no zero-size collapse |
| AC-8.1 | 50,000 rows in the layer panel | Scroll to the middle | Only visible + 8 overscan rows mounted; 60 fps; correct row at `scrollTop/28` |
| AC-8.2 | Layer search `type:FRAME token:color/bg` | Type the query | Results ≤ 200, match both filters, p95 ≤ 50 ms, invalid filter shows inline error |
| AC-9.1 | Two pages with different zoom | Switch back and forth | Each restores pan/zoom; cold load p95 ≤ 1.2 s; skeleton after 200 ms |
| AC-9.2 | Last remaining page | Delete it | Action disabled; no page is removed |
| AC-10.1 | Three nodes with different widths | Focus W and type `200` | All become 200 px; field shows Mixed before the edit; one coalesced event |
| AC-10.2 | A color bound to `color/bg/default` | Edit the color | Configured behavior runs (create variable/detach/ask); binding is restored or replaced atomically |
| AC-11.1 | 10k-node page | Type a fuzzy layer name | Palette returns ≤ 20 results in ≤ 50 ms; Enter selects and reveals; Cmd+Enter adds |
| AC-12.1 | Text editing active | Press `Escape` | Text commits, node selected, palette/menus remain closed; second Escape clears selection |
| AC-12.2 | `prefers-reduced-motion: reduce` | Zoom to selection | No animation; result pulse is static; playback fallback is used |

### 13.2 Edge cases
- Snap candidates at exact ties resolve deterministically; a snap holds for 8 px to prevent flicker.
- Rotated parents: reparent inverts the full matrix; singular matrices cancel, never produce NaN.
- Deleting a node being edited commits/cancels the edit first; the textarea is removed before the node.
- Hidden selected nodes remain selected but render no outline; inspector shows "Hidden" and actions work.
- Locked nodes cannot be dragged, resized, or deleted from canvas; layer panel selection still allows
  unlock, rename, and visibility changes (02 I9).
- A `FILL` child inside a `HUG` parent never collapses to zero; it is treated as `HUG` and warns.
- Deleting the last page is impossible; deleting a page mid-drag cancels the drag and keeps the node.
- Pasting a 10,001-node clipboard is rejected with a clear limit error; nothing partial is inserted.
- Missing fonts never block typing; replacing fonts is one undoable batch across the file.
- A boolean with an empty result and a slice with no parent frame both remain valid, non-rendering nodes.
- Cross-page paste remaps IDs; cross-page drag preserves IDs and page-space coordinates.
- At > 2,000 selected nodes, transform is rejected but selection and inspector remain usable.

### 13.3 Performance budget
| Operation | Target (p50) | Budget (p95) |
|---|---|---|
| Pan/zoom frame | 8 ms | 16.7 ms (60 fps) |
| Drag 1 node / 50 nodes | 6 / 10 ms | 12 / 16.7 ms |
| Selection change (10k page) | 20 ms | 50 ms |
| Hit test | 0.5 ms | 2 ms |
| Auto-layout solve, 5k nodes | 1.2 ms | 2 ms (worker above 2k/page) |
| Text reflow per keystroke | 4 ms | 8 ms |
| Layer search / palette | 20 ms | 50 ms |
| Page switch (cached / cold) | 80 / 600 ms | 100 / 1,200 ms |
| Layer scroll | 8 ms | 16.7 ms |
| File open (current page, 10k nodes) | 800 ms | 1,500 ms |

Instrumentation records frame time, node count, zoom, and degradation events; a budget breach twice in
2 s triggers the LOD/degrade path in §2.7.

---
## 14. Open Questions, Deferred Work, New Identifiers
### 14.1 Open questions
| # | Question | Proposed default |
|---|---|---|
| Q1 | Should guides be shared with collaborators? | No: per-user editor state; revisit after MVP |
| Q2 | Should layer collapse state persist across reloads? | Session-only in MVP; add to `EditorUserState` later |
| Q3 | Does paste-over-selection scale content or only position it? | Scale uniformly, 0.25×–4×, as specified in §3.7 |
| Q4 | Is `sync` selection follow (viewport follows a collaborator) in scope? | Deferred to 04/07 |
| Q5 | Should `SECTION` be a canvas drop target? | Yes, it is a non-clipping container, but no auto layout |
| Q6 | Do text layers need per-range a11y semantics? | No; semantics are node-level in MVP |

### 14.2 Explicitly deferred
- Arabic/Hebrew shaping and full bidi text; RTL layout direction for auto layout.
- Variable-width freeform strokes and pressure profiles stored in the model.
- Arbitrary vector networks (shared vertex across > 2 segments).
- Dragging layer rows onto the canvas; canvas-to-panel drag.
- Full touch parity, radial menus, and hover previews on touch.
- Page-level trash separate from version history; per-page collaborator permissions.
- Boolean operations on `TEXT` and `IMAGE` without an explicit outline/vectorize step.
- mm/in ruler units and document-level unit systems.

### 14.3 New identifiers introduced
**Model / state additions [proposed]** (additive; no existing 02 field changes):
`EditorViewport`, `PageViewport`, `Guide`, `LayoutGrid`, `TextA11y`, `SnapCandidate`, `InsertionTarget`,
`ClipboardDocument` (`kanvas.clipboard/1`), `FontState`, `ToolDefaults`, and the child field
`BaseNode.layoutChild: ChildLayoutProps` (min/max project to `minSize`/`maxSize`). `EditorUserState`
extends `FileMeta.editorState.byUser[userId]` with `pageViewports` and `guidesByPage`. These require a
schemaVersion bump or an editor-state-only migration before implementation.

**Commands introduced** (all `<namespace>.<verb>`):

```
node.create  node.delete  node.duplicate  node.reparent  node.reorder  node.rename
node.setTransform  node.setVisible  node.setLocked  node.setOpacity  node.setBlendMode
node.setClipsContent  node.setCornerRadius  node.setCornerSmoothing  node.setFill  node.setStroke
node.setEffects  node.setLayoutGrids  node.setConstraints  node.setSizing  node.setMinMax
node.setExportSettings  node.setDescription  node.setMask  node.setA11y  node.setArc
node.setPointCount  node.setInnerRadius  node.setPoints  node.group  node.ungroup
node.wrapInFrame  node.moveToPage
page.create  page.rename  page.duplicate  page.reorder  page.delete  page.setBackground
layout.setAutoLayout  layout.removeAutoLayout  layout.setDirection  layout.setSpacing
layout.setPadding  layout.setAlignment  layout.setWrap  layout.setChildLayout
layout.setStrokesIncluded  layout.wrapSelection
text.setCharacters  text.setStyle  text.setRangeStyle  text.setAutoResize  text.setTruncation
text.setMaxLines
vector.setPaths  vector.setVertex  vector.addVertex  vector.removeVertex  vector.breakPath
vector.joinPath  vector.closePath  vector.setFillRule  vector.flatten  vector.outlineStroke
boolean.create  boolean.setOperation
style.apply  style.detach  variable.bind  variable.unbind  variable.create  font.replaceMissing
```
**Engine operations (not DesignCommands):** `history.undo`, `history.redo`.

**UI / read intents (no document mutation):** `view.setViewport`, `view.zoomToFit`,
`view.zoomToSelection`, `view.focusSelection`, `selection.set`, `selection.add`, `selection.remove`,
`selection.toggle`. `solveLayout` remains the read API defined in 02 §7.1.
