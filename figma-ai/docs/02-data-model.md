# 02 — Data Model & Canonical Vocabulary

> **Status:** Normative. This document is the single source of truth for names, types, and invariants.
> Every other document in this spec set must use the identifiers defined here. If a document needs a new
> identifier, it must follow the naming conventions below and be added to the glossary in
> [`README.md`](../README.md).

---

## 1. Purpose

This document defines:

1. The **canonical vocabulary** (the words the whole product, codebase, and AI layer share).
2. The **document graph** (how a design file is represented).
3. The **entity types** for collaboration, history, and AI agents.
4. The **serialization format** (`.kanvas` file), ID scheme, migration rules, and validation invariants.

It is deliberately implementation-shaped: types are given in TypeScript because the product is
TypeScript end-to-end, and JSON examples are given because the file format, the realtime protocol, and
the agent protocol all transport JSON.

Design goal: a node graph that is
**renderable** (Konva can draw it), **diffable** (commands can mutate it), **addressable** (agents can
refer to nodes cheaply), and **migratable** (schema versions can evolve without breaking files).

---

## 2. Canonical Vocabulary

These are the only names used across the product for these concepts. Docs, code, telemetry, and agent
prompts must all use them.

| Canonical name | Meaning | Never call it |
|---|---|---|
| **File** (`KanvasFile`) | A design document. Contains pages, components, styles, variables, assets, comments, history. | project, document (informally OK) |
| **Page** (`Page`) | A named canvas inside a file. Nodes live on exactly one page. | board, sheet, artboard |
| **Node** (`Node`) | Any element in the document graph. Discriminated union, see §6. | layer, object, element |
| **Frame** (`FrameNode`) | A container that clips, is exportable, can be a prototype root. | artboard, screen |
| **Group** (`GroupNode`) | Non-clipping container created by grouping. | folder |
| **Instance** (`InstanceNode`) | A live reference to a `ComponentNode` master. | copy, clone |
| **Component** (`ComponentNode`) | A master node usable as a library definition. | symbol, master (informally OK) |
| **Variant set** (`ComponentSetNode`) | A family of components keyed by variant properties. | variant group |
| **Style** (`Style`) | A named, reusable bundle of paint/effect/text properties. | preset |
| **Variable** (`Variable`) | A named typed value with modes (a design token's storage). | token (a *token* is the user-facing view of a variable) |
| **Token** | User-facing design-system concept backed by `Variable` + `Style` + naming convention. | variable (interchangeably OK in UI copy) |
| **Collection** (`VariableCollection`) | A group of variables that share a set of modes (e.g. `Theme`, `Brand`). | token set |
| **Mode** (`VariableMode`) | A dimension value inside a collection (e.g. `light`, `dark`, `brand-a`). | theme option |
| **Theme** (`Theme`) | A bound selection of one mode per collection. | skin |
| **Asset** (`Asset`) | An uploaded/imported binary (image, SVG, font, icon). | media, file |
| **Interaction** (`Interaction`) | A prototype trigger → action(s) binding on a node. | link, hotspot |
| **Flow** (`Flow`) | A named prototype entry point and its reachable frames. | userflow |
| **Comment thread** (`CommentThread`) | Pinned conversation anchored to canvas coordinates or a node. | annotation |
| **History event** (`HistoryEvent`) | One undoable, attributable change batch. | commit, changeset |
| **Version** (`Version`) | A named snapshot of the whole file. | checkpoint, milestone |
| **Presence** (`PresenceState`) | Ephemeral user/agent state (cursor, selection, tool, viewport). | awareness |
| **Agent** (`Agent`) | A connected AI collaborator identity. | bot, assistant |
| **Agent session** (`AgentSession`) | A live connection of one agent to one file. | connection |
| **Run** (`AgentRun`) | One prompt → finished task execution, with plan, actions, summary. | task, job (task = UI word for run) |
| **Plan** (`AgentPlan`) / **Step** (`AgentPlanStep`) | Ordered, visible intent structure for a run. | todo list |
| **Action** (`AgentAction`) | One recorded agent step effect: tool call, commands, reasoning, result. | tool call, edit |
| **Cursor** (`AgentCursor`) | The visible agent pointer and its state. | pointer |
| **Playback script** (`CursorPlaybackScript`) | The choreography that animates a cursor for an action. | animation |
| **Command** (`DesignCommand`) | The atomic, serializable document mutation primitive. | op (op = the name of a command kind; OK in code) |
| **Envelope** (`CommandEnvelope`) | A transport/attribution wrapper around 1..n commands. | batch (batch = the payload array) |
| **Intent** (`DesignIntent`) | Structured meaning extracted from a natural-language prompt. | parse |
| **Guardrail** (`GuardrailPolicy`) | Permission/safety rules that gate agent actions. | limits |

Terminology rule of thumb: **Node** = data, **Frame** = container, **Instance** = reference,
**Command** = mutation, **Action** = agent-attributable effect, **Event** = history record.

---

## 3. Identifier Scheme

All IDs are string IDs with a type prefix, backed by **UUIDv7** (time-ordered, index-friendly).

```
prefix + "_" + uuidv7            e.g.  n_0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6071
```

| Prefix | Entity | Prefix | Entity |
|---|---|---|---|
| `f_` | File | `ag_` | Agent |
| `pg_` | Page | `as_` | Agent session |
| `n_` | Node | `r_` | Agent run |
| `c_` | Component definition | `p_` | Agent plan |
| `st_` | Agent plan step | `act_` | Agent action |
| `lib_` | Library | `cur_` | Agent cursor (ephemeral, per session) |
| `col_` | Variable collection | `op_` | Command envelope |
| `v_` | Variable | `ev_` | History event |
| `sty_` | Style | `ver_` | Version snapshot |
| `a_` | Asset | `th_` | Comment thread |
| `fl_` | Flow | `cm_` | Comment |
| `u_` | User | `at_` | Audit log entry |
| `int_` | Interaction | `h_` | Agent-local node handle (run-scoped, see §17) |

Rules:

- IDs are **opaque** to clients. Never parse them, never encode semantics in them.
- IDs are stable for the lifetime of an entity. Reparenting, renaming, and restyling never change IDs.
- `h_` handles are the only exception: they are **run-scoped aliases** (`h_F1`, `h_T3`) minted for a
  single agent run to keep LLM context small. They are never persisted and never leak into the command log.

---

## 4. Document Graph

```
KanvasFile
├── pages: Page[]                       (ordered; each Page owns a disjoint node forest)
│   └── nodes: Node[]                   (flat map; parent/child via parentId + children[])
├── components: ComponentDefinition[]   (library-facing metadata for ComponentNode masters)
├── libraries: Library[]                (internal library + linked external libraries)
├── styles: Style[]
├── variableCollections: VariableCollection[]  → variables: Variable[]
├── themes: Theme[]
├── assets: Asset[]
├── flows: Flow[]                       (prototype entry points)
├── commentThreads: CommentThread[]     (soft-deleted, retained for history)
├── agents: AgentRef[]                  (which agents may access; scoped permissions)
└── meta: FileMeta
```

Node hierarchy is stored **flat with explicit parent/child links** (not nested JSON). Reasons:

- O(1) lookup by id for commands, sync, and agent addressing.
- Cheap partial updates in CRDT/realtime layers (one `Y.Map` entry per node).
- Layer panel can be virtualized from the flat map without re-serializing trees.

### 4.1 Invariants (enforced by the Document Engine, see `07-state-model.md`)

| # | Invariant |
|---|---|
| I1 | Every node belongs to exactly one page. Cycles are impossible; `parentId` chains are acyclic. |
| I2 | `children[]` order equals paint order: later = on top. Reordering is a command, not a side effect. |
| I3 | A node's `id` never appears twice in one file. |
| I4 | Container types (`FRAME`, `GROUP`, `COMPONENT`, `COMPONENT_SET`, `INSTANCE`) may have children; leaf types may not. |
| I5 | `COMPONENT_SET` children must all be `COMPONENT` nodes with identical variant axes. |
| I6 | `INSTANCE` nodes have a resolvable `componentId`; if it dangles (deleted library), they become `detached: true` with an inline snapshot (see §11.5). |
| I7 | Coordinates are parent-relative. Root-level nodes are page-relative. Rotation is in radians, clockwise. |
| I8 | `boundVariables` keys are property paths (`fills.0.color`, `itemSpacing`, `textStyle.fontSize`); values are `VariableAlias`. |
| I9 | `visible: false` nodes are still selectable via layer panel, not via canvas. `locked: true` nodes reject direct canvas edits. |
| I10 | Every mutation produces exactly one `HistoryEvent` carrying the inverse commands. |
| I11 | `revision` (int64) is monotonically increasing per file; commands carry `baseRevision`. |
| I12 | Deleted nodes live in a trash bin for 30 days (`deletedAt`, `deletedBy`) so agent deletions are recoverable. |

---

## 5. Core Primitives

```ts
type NodeId = string;
type PageId = string;
type FileId = string;
type AssetId = string;

/** Parent-relative placement. Scale is expressed via width/height; flips via negative scale. */
interface Transform {
  x: number;                 // parent-relative, px
  y: number;                 // parent-relative, px
  width: number;             // px, always >= 0 in the model
  height: number;            // px
  rotation: number;          // radians, clockwise, about the node's center
  scaleX: 1 | -1;            // 1 = normal, -1 = flipped horizontally
  scaleY: 1 | -1;
}

type BlendMode =
  | 'NORMAL' | 'MULTIPLY' | 'SCREEN' | 'OVERLAY' | 'DARKEN' | 'LIGHTEN'
  | 'COLOR_DODGE' | 'COLOR_BURN' | 'HARD_LIGHT' | 'SOFT_LIGHT' | 'DIFFERENCE'
  | 'EXCLUSION' | 'HUE' | 'SATURATION' | 'COLOR' | 'LUMINOSITY'
  | 'PASS_THROUGH';          // containers only

interface RGBA { r: number; g: number; b: number; a: number }   // 0..255, a 0..1

type Paint = SolidPaint | GradientPaint | ImagePaint;

interface PaintBase { visible: boolean; opacity: number; blendMode?: BlendMode }

interface SolidPaint extends PaintBase { type: 'SOLID'; color: RGBA }

interface ColorStop { position: number; color: RGBA }          // position 0..1

interface GradientPaint extends PaintBase {
  type: 'GRADIENT_LINEAR' | 'GRADIENT_RADIAL' | 'GRADIENT_ANGULAR' | 'GRADIENT_DIAMOND';
  gradientTransform: [number, number, number, number, number, number]; // 2x3 affine in unit space
  stops: ColorStop[];
}

interface ImagePaint extends PaintBase {
  type: 'IMAGE';
  assetId: AssetId;
  scaleMode: 'FILL' | 'FIT' | 'CROP' | 'TILE';
  cropTransform?: [number, number, number, number, number, number];
  tileScale?: number;
  filters?: ImageFilters;      // exposure, contrast, saturation, temperature, tint, highlights, shadows
  rotation?: 0 | 90 | 180 | 270;
}

interface Stroke {
  paint: Paint;
  weight: number;                          // px
  align: 'INSIDE' | 'CENTER' | 'OUTSIDE';
  cap: 'BUTT' | 'ROUND' | 'SQUARE';
  join: 'MITER' | 'BEVEL' | 'ROUND';
  dashPattern: number[];
  miterLimit: number;
}

type Effect =
  | { type: 'DROP_SHADOW'   ; color: RGBA; offset: Vec2; radius: number; spread: number; blendMode: BlendMode; visible: boolean }
  | { type: 'INNER_SHADOW'  ; color: RGBA; offset: Vec2; radius: number; spread: number; blendMode: BlendMode; visible: boolean }
  | { type: 'LAYER_BLUR'    ; radius: number; visible: boolean }
  | { type: 'BACKGROUND_BLUR'; radius: number; visible: boolean };

interface Vec2 { x: number; y: number }

interface Corners { topLeft: number; topRight: number; bottomRight: number; bottomLeft: number }

interface Constraints {
  horizontal: 'LEFT' | 'RIGHT' | 'CENTER' | 'LEFT_RIGHT' | 'SCALE';
  vertical: 'TOP' | 'BOTTOM' | 'CENTER' | 'TOP_BOTTOM' | 'SCALE';
}

/** FIXED = keep px, HUG = shrink-wrap content, FILL = take remaining space in parent autolayout */
type SizingAxis = 'FIXED' | 'HUG' | 'FILL';

interface NodeMetadata {
  createdBy: string;             // u_* or ag_*
  createdAt: number;             // epoch ms
  updatedBy: string;
  updatedAt: number;
  revision: number;              // bumped on every command touching this node
  componentKey?: string;         // stable cross-file key for published components
  description?: string;          // agent/user-authored intent, shown in inspect panel
  agentAnnotations?: AgentAnnotation[];
  pluginData?: Record<string, JsonValue>;   // namespaced third-party data
}

interface AgentAnnotation {
  id: string;
  agentId: string;
  runId: string;
  createdAt: number;
  kind: 'RATIONALE' | 'TODO' | 'WARNING' | 'A11Y' | 'HANDOFF';
  text: string;
  resolvedAt?: number;
}

interface NodePermissions {
  /** Who may edit. Absent = inherit from file. */
  editableBy?: string[];
  /** Hard lock: no user, no agent, no publish. Requires owner to unlock. */
  protected?: boolean;
  /** Agents must request confirmation before destructive ops on this subtree. */
  requiresConfirmationForAgents?: boolean;
}
```

---

## 6. Node Type Catalog

```ts
type NodeType =
  | 'FRAME' | 'GROUP' | 'SECTION'
  | 'RECTANGLE' | 'ELLIPSE' | 'LINE' | 'ARROW' | 'POLYGON' | 'STAR' | 'VECTOR' | 'BOOLEAN'
  | 'TEXT' | 'IMAGE'
  | 'COMPONENT' | 'COMPONENT_SET' | 'INSTANCE'
  | 'SLICE';
```

| Type | Children | Key extra props | Notes |
|---|---|---|---|
| `FRAME` | yes | `clipsContent`, `autoLayout?`, `layoutGrids?`, `backgroundBlur`, `isPrototypeStart`, `exportSettings` | Primary artboard & layout container |
| `GROUP` | yes | — | Created by grouping; no fill/clip of its own; bounds are the union of children |
| `SECTION` | yes | `devStatus`, `sectionColor` | Organizational canvas region; excluded from export |
| `RECTANGLE` | no | `cornerRadius: Corners \| number`, `cornerSmoothing: number` | Also used for rounded rects, dividers |
| `ELLIPSE` | no | `arcData?: { startingAngle, endingAngle, innerRadius }` | Donuts/pies |
| `LINE` | no | `points` | 2-point; stroke-only |
| `ARROW` | no | `points`, `arrowStart?`, `arrowEnd?` | Connectors for flows/diagrams |
| `POLYGON` | no | `pointCount` (3..12) | Triangle = 3, diamond = 4 rotated |
| `STAR` | no | `pointCount`, `innerRadius` (0..1) | |
| `VECTOR` | no | `paths: VectorPath[]`, `fillRule`, `windingRule` | Pen/imported SVG/freeform strokes |
| `BOOLEAN` | yes | `operation: 'UNION' \| 'SUBTRACT' \| 'INTERSECT' \| 'EXCLUDE'` | Non-destructive boolean group |
| `TEXT` | no | see §8 | Real text layer with rich ranges |
| `IMAGE` | no | `assetId`, `scaleMode`, `cropTransform`, `filters` | Placed binary asset |
| `COMPONENT` | yes | see §11 | Master |
| `COMPONENT_SET` | yes (only `COMPONENT`) | `variantAxes: { name, values }[]` | Variant family |
| `INSTANCE` | yes (mirrored) | see §11.5 | Live reference |
| `SLICE` | no | `exportSettings` | Export region without visual output |

```ts
interface BaseNode {
  id: NodeId;
  type: NodeType;
  name: string;
  pageId: PageId;
  parentId: NodeId | null;
  children?: NodeId[];
  transform: Transform;
  sizing?: { horizontal: SizingAxis; vertical: SizingAxis };
  minSize?: Partial<Vec2>;
  maxSize?: Partial<Vec2>;
  constraints?: Constraints;
  fills?: Paint[];
  strokes?: Stroke[];
  effects?: Effect[];
  cornerRadius?: number | Corners;
  opacity: number;                    // 0..1
  blendMode: BlendMode;
  visible: boolean;
  locked: boolean;
  isMask?: boolean;
  autoLayout?: AutoLayout;
  layoutGrids?: LayoutGrid[];
  styleRefs?: StyleRef[];             // e.g. [{ styleId, kind: 'FILL', target: 'fills.0' }]
  boundVariables?: Record<string, VariableAlias>;
  absoluteRenderBounds?: Bounds;      // derived cache; not authoritative
  exportSettings?: ExportSetting[];
  metadata: NodeMetadata;
  permissions?: NodePermissions;
  trashedAt?: number;
}

interface FrameNode    extends BaseNode { type: 'FRAME'; clipsContent: boolean; isPrototypeStart?: boolean; backgroundOpacityOverride?: number }
interface GroupNode    extends BaseNode { type: 'GROUP' }
interface SectionNode  extends BaseNode { type: 'SECTION'; sectionColor: string; devStatus?: 'NONE'|'READY'|'IN_PROGRESS' }
interface RectangleNode extends BaseNode { type: 'RECTANGLE'; cornerSmoothing: number }
interface EllipseNode  extends BaseNode { type: 'ELLIPSE'; arcData?: { startingAngle: number; endingAngle: number; innerRadius: number } }
interface LineNode     extends BaseNode { type: 'LINE' | 'ARROW'; points: Vec2[]; arrowStart?: boolean; arrowEnd?: boolean }
interface PolygonNode  extends BaseNode { type: 'POLYGON'; pointCount: number }
interface StarNode     extends BaseNode { type: 'STAR'; pointCount: number; innerRadius: number }
interface VectorNode   extends BaseNode { type: 'VECTOR'; paths: VectorPath[]; fillRule: 'NONZERO'|'EVENODD'; windingRule: 'NONZERO'|'EVENODD' }
interface BooleanNode  extends BaseNode { type: 'BOOLEAN'; operation: 'UNION'|'SUBTRACT'|'INTERSECT'|'EXCLUDE' }
interface TextNode     extends BaseNode { type: 'TEXT'; characters: string; textStyle: TextStyle; rangeStyles: RichTextRange[]; derivedTextData?: TextDerived }
interface ImageNode    extends BaseNode { type: 'IMAGE'; assetId: AssetId; scaleMode: 'FILL'|'FIT'|'CROP'|'TILE'; cropTransform?: Mat2x3; filters?: ImageFilters }
interface SliceNode    extends BaseNode { type: 'SLICE' }
interface ComponentNode extends BaseNode { type: 'COMPONENT'; definitionId: string; variantProperties?: Record<string,string> }
interface ComponentSetNode extends BaseNode { type: 'COMPONENT_SET'; variantAxes: VariantAxis[] }
interface InstanceNode extends BaseNode {
  type: 'INSTANCE';
  componentId: NodeId;
  componentProperties: Record<string, ComponentPropertyValue>;
  overrides: InstanceOverride[];
  detached?: boolean;
  detachedSnapshot?: { componentKey?: string; name: string; nodes: Node[] };
}

type Node = FrameNode | GroupNode | SectionNode | RectangleNode | EllipseNode | LineNode
  | PolygonNode | StarNode | VectorNode | BooleanNode | TextNode | ImageNode | SliceNode
  | ComponentNode | ComponentSetNode | InstanceNode;
```

### 6.1 Vector paths

```ts
interface VectorPath {
  id: string;
  closed: boolean;
  fill: boolean;                  // whether this subpath is filled
  commands: PathCommand[];        // cubic-capable
  /** Editing affordances, not required for rendering: */
  vertices?: Vertex[];
}

type PathCommand =
  | { cmd: 'M'; to: Vec2 }
  | { cmd: 'L'; to: Vec2 }
  | { cmd: 'C'; c1: Vec2; c2: Vec2; to: Vec2 }
  | { cmd: 'Q'; c: Vec2; to: Vec2 }
  | { cmd: 'Z' };

interface Vertex {
  id: string;
  point: Vec2;
  inHandle?: Vec2;                // relative
  outHandle?: Vec2;               // relative
  cornerType: 'STRAIGHT' | 'MIRRORED' | 'ASYMMETRIC' | 'DISCONNECTED';
  // derived by pen tool: 'BEND' corners via cornerSmoothing
}
```

Freeform drawing produces `VECTOR` nodes with many vertices, then a
**simplify** pass (Ramer–Douglas–Peucker at 0.6px tolerance, then curve fit) so strokes stay editable
and small. Imported SVG becomes `VECTOR` nodes with the same structure — no lossy flattening for
paths, groups, and transforms we can map.

---

## 7. Auto Layout

```ts
interface AutoLayout {
  direction: 'HORIZONTAL' | 'VERTICAL';
  wrap: 'NO_WRAP' | 'WRAP';
  itemSpacing: number;                 // gap on primary axis
  counterSpacing?: number;             // gap between wrapped lines
  padding: { top: number; right: number; bottom: number; left: number };
  primaryAxisAlign: 'MIN' | 'CENTER' | 'MAX' | 'SPACE_BETWEEN';
  counterAxisAlign: 'MIN' | 'CENTER' | 'MAX' | 'BASELINE';
  alignContent?: 'MIN' | 'CENTER' | 'MAX' | 'SPACE_BETWEEN';   // wrapped lines
  clipsContent: boolean;
  strokesIncludedInLayout: boolean;
  /** Child-level, stored on the child: */
}
interface ChildLayoutProps {
  layoutGrow: 0 | 1;                   // 1 = fill on primary axis
  layoutAlign: 'INHERIT' | 'MIN' | 'CENTER' | 'MAX' | 'STRETCH';  // cross axis
  layoutPositioning: 'AUTO' | 'ABSOLUTE';   // ABSOLUTE = ignore layout, pinned by x/y
  minWidth?: number; maxWidth?: number; minHeight?: number; maxHeight?: number;
}
```

### 7.1 Layout solver contract

The solver is a pure function, testable in isolation, and identical on client and server:

```ts
function solveLayout(root: NodeId, doc: DocumentView, opts?: { rounding: 'PIXEL' | 'SUB_PIXEL' }): LayoutResult
interface LayoutResult {
  frames: Record<NodeId, { x: number; y: number; width: number; height: number; childOrder: NodeId[]; lines?: NodeId[][] }>;
  overflow: NodeId[];                  // nodes exceeding max constraints (drives UI warning badge)
}
```

Algorithm (single pass, no fixed-point iteration):

1. **Intrinsic pass (post-order):** compute `intrinsicSize` for every node.
   - Leaves: shapes = transform size; `TEXT` with `WIDTH_AND_HEIGHT` = measured text metrics; `IMAGE` HUG = natural aspect from asset.
   - Containers with autolayout and `HUG`: `size = padding + Σ(children) + gaps`, clamped by `min/maxSize`.
2. **Resolve pass (pre-order):** distribute the container's free space.
   - `FILL` children split remaining space equally, then clamped by min/max; if clamped, leftover is
     redistributed to remaining `FILL` children (single redistribution round, deterministic order).
   - `SPACE_BETWEEN` requires ≥2 children; if free space is negative, spacing becomes 0 and the
     container reports `overflow`.
   - `WRAP`: greedy line-breaking along the primary axis using intrinsic sizes; line cross-size =
     max child cross-size; `alignContent` distributes lines.
3. **Place pass:** assign `x/y` with pixel rounding at the last step to avoid cumulative drift.
4. **Absolute children** are skipped by layout and keep their stored `x/y` relative to the container.

Determinism requirement: identical inputs must produce byte-identical `LayoutResult`. This is what makes
agent-generated layouts reproducible and golden-testable.

Complexity: O(n) per layout with a memoized intrinsic pass; only dirty subtrees re-solve. Target
< 2ms for 5,000 nodes on the main thread, or off-main-thread via a worker when a page exceeds 2,000 nodes.

---

## 8. Text Model

```ts
interface TextStyle {
  fontFamily: string;                  // "Inter"
  fontStyle: string;                   // "Regular" | "Medium" | "Semi Bold" | "Bold Italic"
  fontSize: number;                    // px
  lineHeight: { unit: 'PIXELS' | 'PERCENT' | 'AUTO'; value: number };
  letterSpacing: { unit: 'PIXELS' | 'PERCENT'; value: number };
  paragraphSpacing: number;
  paragraphIndent: number;
  textAlignHorizontal: 'LEFT' | 'CENTER' | 'RIGHT' | 'JUSTIFIED';
  textAlignVertical: 'TOP' | 'CENTER' | 'BOTTOM';
  textCase: 'ORIGINAL' | 'UPPER' | 'LOWER' | 'TITLE' | 'SMALL_CAPS' | 'SMALL_CAPS_FORCED';
  textDecoration: 'NONE' | 'UNDERLINE' | 'STRIKETHROUGH';
  textAutoResize: 'NONE' | 'WIDTH_AND_HEIGHT' | 'HEIGHT' | 'TRUNCATE';
  truncation: 'DISABLED' | 'ENDING';
  maxLines?: number;
  listStyle: 'NONE' | 'ORDERED' | 'UNORDERED' | 'TODO';
  listSpacing: number;
  hangingPunctuation: boolean;
  hangingList: boolean;
  fills: Paint[];                      // text color lives here (Figma-compatible mental model)
  hyperlink?: { type: 'URL'; url: string };
}

/** Sparse rich-text: base style comes from node.textStyle, ranges override specific properties. */
interface RichTextRange {
  start: number;                       // UTF-16 index, inclusive
  end: number;                         // exclusive
  style: Partial<Omit<TextStyle, 'textAutoResize'>>;
}

interface TextDerived {                // engine-computed cache, safe to drop
  glyphCount: number;
  lineCount: number;
  baselineOffsets: number[];            // per line, for baseline alignment
  measuredWidth: number;
  measuredHeight: number;
  overflowLines: number;
}
```

Invariants:

- `rangeStyles` ranges never overlap; they are normalized (merged/split) after every edit.
- `characters` is the source of truth; `derivedTextData` is recomputed on font load, edit, or resize.
- Fonts are loaded via `FontRegistry`; until a font resolves, text renders with a fallback metric and
  is marked `fontState: 'LOADING'` in the UI (never blocks input).
- Editing an `INSTANCE`'s text writes an override, never the master (see §11.4).

---

## 9. Styles, Variables, Themes (Design System Layer)

```ts
type StyleKind = 'FILL' | 'STROKE' | 'EFFECT' | 'TEXT' | 'GRID';

interface Style {
  id: string;                          // sty_*
  name: string;
  kind: StyleKind;
  description?: string;
  definition: Paint[] | Stroke[] | Effect[] | TextStyle | LayoutGrid[];
  boundVariables?: Record<string, VariableAlias>;
  libraryId?: string;                  // set when published
  publishedVersion?: string;
  remote?: boolean;                    // came from a linked library
  createdAt: number; updatedAt: number; updatedBy: string;
}

interface StyleRef { styleId: string; kind: StyleKind; target: string }  // target = a property path

type VariableResolvedType = 'COLOR' | 'FLOAT' | 'STRING' | 'BOOLEAN';

interface VariableCollection {
  id: string;                          // col_*
  name: string;                        // "Theme", "Brand", "Density"
  modes: VariableMode[];               // ordered; first = default
  defaultModeId: string;
  isLibrary?: boolean;
  libraryId?: string;
  variableIds: string[];
  hiddenFromPublishing?: boolean;
}

interface VariableMode { id: string; name: string }   // "light", "dark", "brand-a"

interface Variable {
  id: string;                          // v_*
  key: string;                         // stable across files for library sync
  name: string;                        // "color/bg/surface/raised"  (slash = token hierarchy)
  collectionId: string;
  resolvedType: VariableResolvedType;
  valuesByMode: Record<string, VariableValue>;
  scopes: VariableScope[];             // where it may be bound; drives the picker UI
  codeSyntax?: { web?: string; android?: string; ios?: string; flutter?: string };
  category: TokenCategory;             // derived from name OR explicit; used by panels & export
  description?: string;
  hiddenFromPublishing?: boolean;
  createdAt: number; updatedAt: number; updatedBy: string;
}

type VariableValue = RGBA | number | string | boolean | VariableAlias;
interface VariableAlias { type: 'VARIABLE_ALIAS'; id: string }   // alias = semantic token referring to a primitive

type VariableScope =
  | 'ALL_SCOPES' | 'TEXT_CONTENT' | 'CORNER_RADIUS' | 'WIDTH_HEIGHT' | 'GAP'
  | 'ALL_FILLS' | 'FRAME_FILL' | 'SHAPE_FILL' | 'TEXT_FILL' | 'STROKE_COLOR'
  | 'EFFECT_COLOR' | 'FONT_FAMILY' | 'FONT_STYLE' | 'FONT_WEIGHT' | 'FONT_SIZE'
  | 'LINE_HEIGHT' | 'LETTER_SPACING' | 'PARAGRAPH_SPACING' | 'OPACITY' | 'STROKE_FLOAT';

type TokenCategory =
  | 'color' | 'typography' | 'space' | 'radius' | 'shadow' | 'elevation'
  | 'border' | 'motion' | 'opacity' | 'size' | 'other';

interface Theme {
  id: string;
  name: string;                        // "Light", "Dark", "Brand A — Dark"
  /** one mode per collection */
  modeByCollection: Record<string, string>;
  /** local overrides applied on top of modes (rare; for one-off brand tweaks) */
  overrides?: Record<string, VariableValue>;   // variableId -> value
  isDefault?: boolean;
}
```

### 9.1 Resolution algorithm (must be identical in editor, player, and exporters)

```
resolve(variableId, theme):
  1. if theme.overrides[variableId] exists -> return it
  2. mode = theme.modeByCollection[collectionId] ?? collection.defaultModeId
  3. value = variable.valuesByMode[mode]
  4. if value is VariableAlias -> recurse (cycle detection: max depth 32, cycle = error, render magenta)
  5. return value
```

Binding rules:

- A node property is **bound** when `boundVariables[path] = alias`. Bound properties are read-only in
  the properties panel until unbound (a "detach token" affordance is always available).
- Editing a bound property *creates a new variable* at the current value (Figma-style flow) or detaches,
  per the user's preference setting `tokenEditBehavior: 'CREATE_VARIABLE' | 'DETACH' | 'ASK'`.
- Semantic tokens: `category` is derived from the name path. Prefix conventions:
  `color/*`, `space/*`, `radius/*`, `border/*`, `shadow/*`, `elevation/*`, `typography/*`, `motion/*`.
  A token whose value is an alias to another token's variable and whose name starts with a
  role word (`bg`, `fg`, `border`, `accent`, `danger`, …) is treated as **semantic**; primitives are
  detected as leaves in the alias graph without role prefixes.
- **Component tokens** (`button/primary/bg/default`) are ordinary variables whose `scopes` and
  `category` are component-scoped; they may alias semantic tokens, never primitives directly (linted).

---

## 10. Assets

```ts
type AssetKind = 'IMAGE' | 'SVG' | 'ICON' | 'ILLUSTRATION' | 'FONT' | 'VIDEO';

interface Asset {
  id: string;                          // a_*
  kind: AssetKind;
  name: string;
  hash: string;                        // sha256 of bytes -> dedupe key
  mimeType: string;
  bytes: number;
  width?: number; height?: number;
  /** Storage layout in Supabase Storage: files/{fileId}/assets/{hash[0:2]}/{hash}.{ext} */
  storageKey: string;
  thumbnailKey?: string;               // 256px webp
  optimized?: { key: string; codec: 'webp' | 'avif'; quality: number; bytes: number };
  sourceUrl?: string;
  tags: string[];
  folderId?: string;
  createdBy: string; createdAt: number;
  usage: { nodeIds: NodeId[] };        // maintained by the engine, used for "replace globally"
  license?: { name: string; url?: string; attribution?: string };
  remoteLibraryId?: string;            // when pulled from team library
}
```

Rules: content-addressed dedupe by `hash`; SVG assets are parsed once into `VectorPath` trees cached
under `derived.svgAst`; fonts are registered by family/style and never inlined into the doc JSON.

---

## 11. Components, Variants, Instances

### 11.1 Definitions, keys, libraries

```ts
interface ComponentDefinition {
  id: string;                          // c_*
  key: string;                         // stable, content-independent identity across files
  name: string;                        // "Button/Primary"
  description?: string;
  nodeId: NodeId;                      // the COMPONENT node (or COMPONENT_SET) in this file
  componentSetId?: string;
  variantProperties?: Record<string, string>;    // { size: 'md', state: 'default' }
  propertyDefinitions: ComponentPropertyDefinition[];
  libraryId?: string;
  publishedVersion?: string;
  semver?: string;                     // "1.4.0" while published
  createdAt: number; updatedAt: number;
}

interface VariantAxis { name: string; values: string[] }   // e.g. { name:'state', values:['default','hover','pressed','disabled','loading'] }

interface ComponentPropertyDefinition {
  key: string;                         // stable, e.g. "#12:0"
  name: string;                        // "label", "showIcon", "state", "leadingIcon"
  type: 'BOOLEAN' | 'TEXT' | 'INSTANCE_SWAP' | 'VARIANT' | 'NUMBER';
  defaultValue: ComponentPropertyValue;
  variantOptions?: string[];           // VARIANT only
  preferredValues?: { type: 'COMPONENT' | 'COMPONENT_SET'; key: string }[];  // INSTANCE_SWAP only
  description?: string;
}

type ComponentPropertyValue = boolean | string | number | { componentKey: string };

interface Library {
  id: string;                          // lib_*
  name: string;
  kind: 'INTERNAL' | 'TEAM' | 'EXTERNAL';
  publishedAt?: number;
  version?: string;                    // semver of the latest publish
  changelog?: string;
  items: { componentKeys: string[]; styleIds: string[]; collectionIds: string[] };
  linkedByFiles: FileId[];
  updatePolicy: 'AUTO_ACCEPT_PATCH' | 'NOTIFY' | 'MANUAL';
}
```

### 11.2 Publishing & versioning

- Publishing produces a `LibraryPublish`: semver (patch = visual-only changes, minor = new
  components/properties, major = breaking property/structural changes), a diff summary
  (added/removed/changed), and an immutable artifact snapshot for rollback.
- Component version history is stored as an append-only list of
  `{ version, publishedAt, publishedBy, diffSummary, snapshotRef }` per component key.
- Consumers see an **update badge** on instances whose master has a newer minor/major; "Accept update"
  applies a three-way merge: (base published snapshot, local overrides, new master).

### 11.3 Variants

- A `COMPONENT_SET` contains `COMPONENT` children; the axis matrix must be rectangular (missing
  combinations are auto-created as empty masters, never silently dropped).
- Variant names follow `Property=Value, Property2=Value2`.
- `INSTANCE` of a set picks a combination through its `state`/variant properties; unknown combos fall
  back to the default variant and surface a warning badge.
- Default interaction states shipped in the starter library:
  `default | hover | pressed | focus | disabled | loading | error`.

### 11.4 Overrides

```ts
interface InstanceOverride {
  /** Path from the instance root to the affected node, e.g. ["card","title"] or ["button","label"] */
  nodePath: string[];
  property: string;                    // 'characters' | 'fills.0.color' | 'visible' | 'componentId' | ...
  value: JsonValue;
  mode: 'SET' | 'REMOVE';
  /** override that only applies when a variant/property condition holds */
  when?: { propertyKey: string; equals: ComponentPropertyValue };
  authoredBy: string;                  // u_* | ag_*
  authoredAt: number;
}
```

Rules:

- Overrides are stored **sparsely and path-based**, so master edits propagate everywhere else
  (this is what makes library updates sane, and what lets agents edit one instance without touching a master).
- `nodePath` is resolved against the instance's mirrored structure; unresolvable paths become
  `orphanedOverrides` (kept, surfaced in a cleanup panel, never auto-deleted).
- **Protected masters:** `NodePermissions.protected` on a `COMPONENT` means agents cannot edit it and
  humans must unlock; publish requires unlocked state. Agents editing an instance always write overrides.

### 11.5 Detach & dangling masters

Detach copies the master subtree into the instance (keeping IDs from an `detachedSnapshot`), rewriting
overrides into concrete values. If a linked library is unlinked or a master deleted, instances keep a
`detachedSnapshot` and render normally, flagged `DANGLING` in the UI with a "restore link" action.

---

## 12. Prototyping

```ts
interface Flow { id: string; name: string; pageId: PageId; startNodeId: NodeId; description?: string }

interface Interaction {
  id: string;                          // int_*
  nodeId: NodeId;
  trigger: Trigger;
  actions: Action[];                   // executed in order
  transition?: Transition;
  overlay?: OverlaySettings;
  enabled: boolean;
  order: number;                       // deterministic evaluation order for overlapping hit areas
}

type Trigger =
  | { type: 'ON_CLICK' }
  | { type: 'ON_HOVER'; delayMs?: number }
  | { type: 'ON_PRESS' }
  | { type: 'ON_DRAG'; direction: 'LEFT'|'RIGHT'|'UP'|'DOWN'|'ANY'; thresholdPx: number }
  | { type: 'ON_KEY'; key: string; modifiers?: ('META'|'CTRL'|'SHIFT'|'ALT')[] }
  | { type: 'AFTER_TIMEOUT'; timeoutMs: number }
  | { type: 'ON_SCROLL'; direction: 'UP'|'DOWN'|'ANY'; thresholdPx: number }
  | { type: 'ON_MOUSE_ENTER' } | { type: 'ON_MOUSE_LEAVE' };

type Action =
  | { type: 'NAVIGATE'; destinationId: NodeId; resetScroll?: boolean }
  | { type: 'BACK' }
  | { type: 'CLOSE_OVERLAY' }
  | { type: 'OPEN_LINK'; url: string; openInNewTab: boolean }
  | { type: 'SCROLL_TO'; targetId: NodeId; behavior: 'SMOOTH'|'INSTANT'; offset?: Vec2 }
  | { type: 'SET_VARIABLE'; variableId: string; modeId?: string; value: VariableValue }
  | { type: 'CHANGE_VARIANT'; nodeId: NodeId; propertyKey: string; value: ComponentPropertyValue }
  | { type: 'SET_STATE'; nodeId: NodeId; property: string; value: JsonValue }   // visibility/opacity toggles
  | { type: 'NODE_ACTIONS'; actions: Action[] }                                  // nested sequence
  | { type: 'AI_GENERATE'; prompt: string; targetFrameId: NodeId }               // runtime agent invocation
  | { type: 'AI_VARIANT'; targetFrameId: NodeId; instruction: string; variants: number };

interface Transition {
  type: 'INSTANT' | 'DISSOLVE' | 'SMART_ANIMATE' | 'MOVE_IN' | 'MOVE_OUT' | 'PUSH' | 'SLIDE_IN' | 'SLIDE_OUT';
  direction?: 'LEFT'|'RIGHT'|'UP'|'DOWN';
  durationMs: number;                  // 0..10000
  easing: EasingSpec;
}
type EasingSpec =
  | { type: 'LINEAR' | 'EASE_IN' | 'EASE_OUT' | 'EASE_IN_OUT' | 'GENTLE' | 'QUICK' | 'BOUNCY' | 'SLOW' }
  | { type: 'CUSTOM_CUBIC'; points: [number, number, number, number] }
  | { type: 'SPRING'; stiffness: number; damping: number; mass: number };

interface OverlaySettings {
  position: 'CENTER'|'TOP_LEFT'|'TOP_CENTER'|'TOP_RIGHT'|'BOTTOM_LEFT'|'BOTTOM_CENTER'|'BOTTOM_RIGHT'|'MANUAL';
  manualPosition?: Vec2;
  background: { type: 'NONE' | 'SOLID'; color: RGBA };
  dismissOnClickOutside: boolean;
  keepScrollPosition: boolean;
}
```

**Smart Animate matching** (must be specified or it becomes voodoo):

1. Match nodes **within** the two frames by, in order: identical `id` (if the frame was duplicated) →
   matching `componentKey` → matching `name` + type → nearest size similarity above 0.85.
2. Matched pairs tween: transform (x/y/width/height/rotation), opacity, fills (only if solid→solid; else cross-fade).
3. Unmatched outgoing nodes fade out; unmatched incoming nodes fade in.
4. Easing applies to the whole timeline; nested transform origins use node centers.

Runtime semantics are defined in `04-feature-spec-systems.md` §Prototyping; prototype evaluation is a
pure reducer over document + variable state so that "AI-generated prototype" and manual prototype behave identically.

---

## 13. Pages, File Meta, Versioning

```ts
interface Page {
  id: string;                          // pg_*
  fileId: string;
  name: string;
  index: number;
  rootNodeIds: NodeId[];
  background: { type: 'SOLID' | 'NONE'; color: RGBA };
  isCurrent?: boolean;                 // ephemeral client flag
  createdAt: number; updatedAt: number;
}

interface FileMeta {
  id: string;                          // f_*
  name: string;
  workspaceId: string;
  ownerId: string;                     // u_*
  createdAt: number; updatedAt: number;
  schemaVersion: number;               // current: 1
  revision: number;                    // int64, monotonic
  thumbnailKey?: string;
  editorState: {                       // last-view state, per user
    byUser: Record<string, { pageId: string; zoom: number; viewport: Vec2; selection: NodeId[] }>;
  };
  agentPolicy: GuardrailPolicy & { allowedAgents: string[] };
  sharing: { visibility: 'PRIVATE'|'LINK'|'WORKSPACE'; linkRole?: 'VIEW'|'COMMENT'|'EDIT'; }
  defaultThemeId?: string;
}

interface Version {
  id: string;                          // ver_*
  fileId: string;
  name: string;                        // "Login flow v2"
  description?: string;
  createdBy: string;                   // u_* | ag_*
  createdAt: number;
  revision: number;
  snapshotRef: string;                 // storage key of a full snapshot (compressed JSON)
  auto?: boolean;                      // auto-created (e.g. before an agent run)
  agentRunId?: string;                 // set when created by an agent
}
```

Snapshot policy: full snapshot on manual version, on publish, and before any agent run that touches
> 25 nodes; otherwise incremental `HistoryEvent` deltas are kept (see `07-state-model.md` §5).

---

## 14. Collaboration Entities

```ts
interface PresenceState {
  actorId: string;                     // u_* | ag_*
  actorType: 'HUMAN' | 'AGENT';
  displayName: string;
  color: string;                       // identity color (see 01-ux-architecture.md §9.3)
  avatarUrl?: string;
  pageId: string;
  cursor?: { x: number; y: number };   // canvas space
  selection: NodeId[];
  hoveredNodeId?: NodeId;
  activeTool?: string;
  viewport?: { x: number; y: number; zoom: number };
  isTyping?: boolean;
  isDragging?: boolean;
  agent?: { sessionId: string; status: AgentRuntimeState; label?: string; runId?: string };
  updatedAt: number;
  /** monotonic per-actor sequence so receivers can drop out-of-order updates */
  seq: number;
}

interface CommentThread {
  id: string;                          // th_*
  fileId: string; pageId: string;
  anchorNodeId?: NodeId;               // node-anchored (moves with node) …
  position: Vec2;                      // … and always stores canvas coordinates as fallback
  resolved: boolean;
  resolvedBy?: string; resolvedAt?: number;
  comments: string[];                  // cm_* ids, ordered
  createdBy: string; createdAt: number;
  pinned?: boolean;
}

interface Comment {
  id: string;                          // cm_*
  threadId: string;
  body: string;                        // markdown subset
  mentions: string[];                  // u_* | ag_*  (mentioning an agent creates an agent request!)
  attachments: AssetId[];
  parentCommentId?: string;
  reactions: { emoji: string; actorIds: string[] }[];
  createdBy: string; createdAt: number; editedAt?: number;
}

interface HistoryEvent {
  id: string;                          // ev_*
  fileId: string; pageId: string;
  actorId: string;                     // u_* | ag_*
  actorType: 'HUMAN' | 'AGENT' | 'SYSTEM';
  origin: 'USER' | 'AGENT' | 'IMPORT' | 'LIBRARY_UPDATE' | 'PROTOTYPE' | 'SYSTEM';
  label: string;                       // "Create login button", "agent: apply auto layout"
  intent?: string;                     // the human/agent intent behind it, verbatim
  agentRunId?: string;
  planStepId?: string;
  envelopeId: string;                  // op_*
  changedNodeIds: NodeId[];
  inverse: DesignCommand[];            // commands that undo this event
  revisionBefore: number; revisionAfter: number;
  ts: number;
  batchId?: string;                    // groups a run's events into one logical change set
  coalesceKey?: string;                // e.g. "drag:n_123:transform" — merges live drag history
  undoable: boolean;
}

interface AuditLogEntry {
  id: string;                          // at_*
  fileId: string; actorId: string; actorType: 'HUMAN' | 'AGENT' | 'SYSTEM';
  runId?: string; actionId?: string;
  kind: 'COMMAND' | 'PERMISSION' | 'CONFIRMATION' | 'GUARDRAIL_BLOCK' | 'EXPORT' | 'PUBLISH' | 'CONNECTION';
  summary: string;
  payload: JsonValue;                   // command summary, grant, denial reason, export config …
  ts: number;
  immutable: true;
}
```

Notes:

- `PresenceState` is ephemeral: it is never written to Postgres, never versioned, and dropped after 30s of silence.
- `Comment` mentioning an agent (`@claude`) creates an `AgentRun` whose prompt is the comment body with
  thread context attached — this is the bridge between human collaboration and the agent layer.
- `AuditLogEntry` is append-only and separately retained (compliance, trust, "why did this change?").

---

## 15. Agent Entities

```ts
type AgentRuntimeState =
  | 'OFFLINE' | 'CONNECTING' | 'STANDBY' | 'THINKING' | 'PLANNING'
  | 'WAITING_APPROVAL' | 'EXECUTING' | 'PAUSED' | 'BLOCKED'
  | 'ERROR' | 'COMPLETED';

type AgentKind = 'BUILTIN' | 'CLAUDE_CODE' | 'CODEX' | 'MCP_CLIENT' | 'CUSTOM';

interface Agent {
  id: string;                          // ag_*
  kind: AgentKind;
  name: string;                        // "Claude Code (frontend)", "Codex (a11y)"
  providerId: 'anthropic' | 'openai' | 'local' | 'custom';
  model?: string;
  color: string;                       // identity color, unique per agent in a file
  avatar?: { kind: 'ICON'|'IMAGE'|'INITIALS'; value: string };
  capabilities: AgentCapability[];     // what this agent is allowed/able to do
  scopes: PermissionScope[];
  persona?: { role: string; systemPromptRef?: string; style?: string };
  createdAt: number;
  createdBy: string;
  defaultCursorStyle?: 'FULL' | 'MINIMAL' | 'GHOST';
}

type AgentCapability =
  | 'DESIGN_CREATE' | 'DESIGN_EDIT' | 'STYLE_APPLY' | 'TOKEN_MANAGE' | 'COMPONENT_MANAGE'
  | 'PROTOTYPE_LINK' | 'ASSET_MANAGE' | 'EXPORT_RUN' | 'A11Y_AUDIT' | 'CODE_GEN'
  | 'PAGE_MANAGE' | 'COMMENT_WRITE' | 'VERSION_CREATE' | 'DESTRUCTIVE';

type PermissionScope =
  | 'read' | 'write' | 'destructive' | 'publish' | 'export' | 'manage_tokens' | 'manage_assets';

interface AgentSession {
  id: string;                          // as_*
  agentId: string;
  fileId: string;
  transport: 'WEBSOCKET' | 'LOCAL_BRIDGE' | 'MCP';
  state: AgentRuntimeState;
  startedAt: number;
  lastHeartbeat: number;
  clientInfo?: { app: string; version: string; os?: string };
  activeRunId?: string;
  cursorId?: string;                   // cur_*
  grantedScopes: PermissionScope[];
  grantedAt: number;
  expiresAt?: number;
}

interface AgentRun {
  id: string;                          // r_*
  sessionId: string; agentId: string; fileId: string; pageId: string;
  prompt: string;
  promptAttachments: { kind: 'NODE' | 'ASSET' | 'TEXT' | 'SCREENSHOT'; ref: string; label: string }[];
  intent?: DesignIntent;
  plan?: AgentPlan;
  state: AgentRuntimeState;
  startedAt: number; endedAt?: number;
  actions: string[];                   // act_* ids in order
  stepIndex: number;
  checkpoints: { at: number; revision: number; label: string }[];
  usage: { inputTokens: number; outputTokens: number; cachedTokens: number; costUsd: number; modelCalls: number };
  summary?: AgentRunSummary;
  parentRunId?: string;                // for sub-agent / multi-agent delegation
  delegatedTo?: string[];              // run ids created by this run (multi-agent)
}

interface AgentPlan {
  id: string;                          // p_*
  runId: string;
  goal: string;
  assumptions: string[];
  steps: AgentPlanStep[];
  approval: { required: boolean; reason?: string; approvedBy?: string; approvedAt?: number; editedByUser?: boolean };
  createdAt: number; updatedAt: number;
}

interface AgentPlanStep {
  id: string;                          // st_*
  index: number;
  title: string;                       // "Add email and password fields"
  detail?: string;
  intent: string;                      // why this step exists
  tools: string[];                     // planned tool names
  targetNodeIds: NodeId[];             // resolved handles at plan time
  dependsOn: string[];                 // step ids
  status: 'PENDING' | 'RUNNING' | 'DONE' | 'SKIPPED' | 'FAILED' | 'NEEDS_REVIEW' | 'REVERTED';
  estimateMs?: number;
  actualMs?: number;
  artifactNodeIds?: NodeId[];          // what the step produced
}

type AgentActionKind =
  | 'PLAN' | 'TOOL_CALL' | 'COMMAND_BATCH' | 'CURSOR_PLAYBACK' | 'MESSAGE' | 'ASK_USER'
  | 'CONFIRM_REQUEST' | 'CHECKPOINT' | 'REVIEW' | 'SUMMARY' | 'ERROR' | 'DELEGATE';

interface AgentAction {
  id: string;                          // act_*
  runId: string; stepId?: string;
  agentId: string;
  kind: AgentActionKind;
  /** LLM-facing tool name, when kind = TOOL_CALL */
  tool?: string;                       // "kanvas.create_frame"
  args?: JsonValue;
  /** Resolved design commands, when kind = COMMAND_BATCH (1..n) */
  commands?: DesignCommand[];
  result?: { ok: boolean; changedNodeIds: NodeId[]; revision: number; warnings: string[]; error?: CommandError };
  /** Human-readable explainability, mandatory for TOOL_CALL and COMMAND_BATCH */
  reasoning: string;                   // "The screen needs a mobile canvas, so I create a 390x844 frame."
  confidence?: number;                 // 0..1, surfaced in the timeline
  status: 'PENDING' | 'RUNNING' | 'DONE' | 'FAILED' | 'BLOCKED' | 'SKIPPED';
  cursorScriptId?: string;             // cur_* playback script shown to humans
  startedAt: number; endedAt?: number;
  reversible: boolean;
  confirmation?: { required: boolean; reason: string; grantedBy?: string; grantedAt?: number; status: 'NOT_REQUIRED'|'PENDING'|'GRANTED'|'DENIED' };
  auditId?: string;
  tokens?: { input: number; output: number };
}

interface AgentRunSummary {
  headline: string;                     // "Login form ready — 7 elements, auto layout applied"
  what: string[];                       // bullets: what was created/changed
  why: string[];                        // rationale highlights
  nodeIds: NodeId[];
  warnings: string[];                   // overflow, contrast, missing font, low confidence
  nextSuggestions: string[];            // "Add social login", "Check dark theme contrast"
  exportHints?: { target: 'react'|'html'|'tailwind'; ready: boolean; notes?: string }[];
}
```

### 15.1 Cursor & playback

```ts
type CursorVisualState =
  | 'IDLE' | 'MOVING' | 'HOVER' | 'PRESS' | 'DRAG' | 'DROP' | 'TYPE' | 'RESIZE'
  | 'SELECT_BOX' | 'WAIT' | 'THINKING' | 'ERROR' | 'DONE';

interface AgentCursor {
  id: string;                          // cur_* (ephemeral)
  agentId: string; sessionId: string;
  state: CursorVisualState;
  /** authoritative canvas-space position */
  position: Vec2;
  /** where it is heading, used for interpolation on the receiver side */
  target?: Vec2;
  targetNodeId?: NodeId;
  label: string;                       // "Claude Code"
  labelMode: 'NAME' | 'STATUS' | 'TOOL' | 'NONE';
  tooltip?: { text: string; kind: 'MESSAGE'|'TOOL'|'WARNING'|'QUESTION'; ttlMs?: number };
  dragTrail?: { points: Vec2[]; startedAt: number };   // for drag visualization
  selectionBox?: { from: Vec2; to: Vec2 };             // for box-select visualization
  color: string;
  seq: number;
  updatedAt: number;
}

interface CursorPlaybackScript {
  id: string;                          // cur_* run-scoped
  agentId: string; runId: string; actionId?: string;
  keyframes: CursorKeyframe[];
  totalDurationMs: number;
  interruptible: boolean;
  speedMultiplier: number;             // user setting: 0.5 | 1 | 2 | instant
  reducedMotionFallback: 'FADE_LABEL' | 'JUMP_TAGGED' | 'SKIP';
  /** Node highlight pulse after the action lands; makes "what changed" unmissable. */
  resultPulse?: { nodeIds: NodeId[]; kind: 'CREATE'|'UPDATE'|'DELETE'|'LAYOUT'; durationMs: number };
}

interface CursorKeyframe {
  t: number;                           // ms offset from script start
  to: Vec2;                            // canvas space destination
  state: CursorVisualState;
  label?: string; tooltip?: CursorTooltip;
  easing?: MotionEasing;
  holdMs?: number;                     // deliberate pause before the next keyframe
  press?: { nodeId?: NodeId; at: Vec2 } | null;
}
```

Full motion rules, timing curves, and reduced-motion behavior: `05-ai-agent-layer.md` §6.

### 15.2 Intent & Guardrails

```ts
interface DesignIntent {
  version: 1;
  requestId: string;
  locale: string;                      // BCP-47, e.g. "id-ID"
  rawPrompt: string;
  summary: string;                     // one-line restatement shown back to the user
  target: {
    scope: 'NEW_FRAME' | 'CURRENT_SELECTION' | 'NODE' | 'PAGE' | 'FILE';
    pageId?: string; nodeIds?: NodeId[];
    framePreset?: 'MOBILE' | 'TABLET' | 'DESKTOP' | 'WATCH' | 'CUSTOM';
    frameSize?: { width: number; height: number };
  };
  theme: {
    mood: string[];                    // ["modern","minimal","premium","calm"]
    density: 'COMPACT' | 'COMFORTABLE' | 'SPACIOUS';
    colorway: 'LIGHT' | 'DARK' | 'BOTH';
    styleKeywords: string[];
    themeId?: string;                  // existing theme to reuse
  };
  layout: { pattern: string; sections: string[]; responsive: boolean; useAutoLayout: boolean };
  elements: IntentElement[];
  behavior: {
    useTokens: boolean; useComponents: boolean; prototyping: 'NONE'|'BASIC'|'FULL';
    accessibilityTarget: 'NONE'|'AA'|'AAA'; motion: 'NONE'|'SUBTLES';
  };
  constraints: { maxSteps?: number; forbiddenOps?: string[]; protectedNodeIds?: NodeId[]; budget?: { tokens?: number; usd?: number } };
  deliverable: { exportAssets?: ('PNG'|'SVG'|'PDF')[]; exportCode?: 'REACT'|'TAILWIND'|'HTML'|'FLUTTER'|'RN'; report: boolean };
  ambiguities: { question: string; options: string[]; default: string }[];
}

interface IntentElement {
  role: 'LOGO'|'TITLE'|'SUBTITLE'|'INPUT'|'PASSWORD'|'BUTTON'|'LINK'|'DIVIDER'|'ICON'|'IMAGE'|'CARD'|'LIST'|'NAV'|'TAB'|'MODAL'|'TABLE'|'AVATAR'|'BADGE'|'OTHER';
  label?: string; placeholder?: string; helper?: string; value?: string;
  required?: boolean; componentHint?: string;          // "Button/Primary"
  tokenHints?: { fill?: string; textFill?: string; radius?: string; spacing?: string };
  sizeHint?: 'SM'|'MD'|'LG';
  props?: Record<string, JsonValue>;
}

interface GuardrailPolicy {
  maxNodesPerRun: number;              // default 400
  maxCommandsPerStep: number;          // default 50
  maxRunDurationMs: number;            // default 15 * 60_000
  maxCostUsdPerRun: number;            // default 2.00
  destructiveRequiresConfirmation: boolean;   // default true
  requirePlanApproval: 'NEVER'|'LARGE'|'ALWAYS';  // LARGE = > 12 steps or destructive present
  protectedNodeIds: NodeId[];          // locked by default: component masters, tokens, brand assets
  protectedPaths: string[];            // e.g. "pages/*/frames/*/logo" style globs
  forbiddenOps: string[];              // e.g. ["node.delete", "component.publish"]
  allowedPages?: PageId[];
  allowNetwork: boolean;               // if false, agent cannot fetch remote assets/providers
  requireAnnotationOnMasters: boolean; // agent must annotate rationale when touching a master
}
```

---

## 16. Query, Addressing, and Context Compression

Agents are token-limited; nodes are verbose. The engine therefore exposes a **three-tier read API**:

| Tier | Call | Returns | Typical size |
|---|---|---|---|
| T0 | `getSelection()` | ids, types, names, sizes, positions | ~40 tokens/node |
| T1 | `queryNodes(selector)` | filtered compact rows | ~60 tokens/node |
| T2 | `getNode(id, depth)` | full JSON subtree (default depth 2) | ~250+ tokens/node |

```ts
type NodeSelector = {
  pageId?: string;
  type?: NodeType | NodeType[];
  name?: { contains?: string; matches?: string };  // glob
  inFrame?: NodeId;                                // descendants of
  hasAutoLayout?: boolean;
  hasBoundVariable?: boolean;
  componentKey?: string;
  isInstanceOf?: string;
  tag?: string;
  limit?: number;                                  // default 50, max 200
};

interface CompactNode {
  h: string;                 // run-scoped handle, e.g. "h_F1"
  id: NodeId;
  t: NodeType;
  name: string;
  box: [x: number, y: number, w: number, h: number];
  layout?: 'H' | 'V' | 'NONE';               // autolayout direction
  kids?: number;                             // child count
  text?: string;                             // first 60 chars
  tokens?: string[];                         // bound variable names
  flags?: ('locked'|'hidden'|'instance'|'component'|'overflow')[];
}
```

Handle rules:

- A run receives handles for nodes it touches or queries; handles are stable for the run.
- `h_*` handles are accepted by every agent tool and resolved to `n_*` by the gateway. Agents may also
  use raw `n_*` ids.
- Cursors/handles are dropped when a run ends; nothing persists them.

---

## 17. Serialization: the `.kanvas` File Format

A file is a **ZIP container** (`.kanvas`), content-addressed and streamable. Live editing stores the same
projection in Postgres + object storage; `.kanvas` is the portable artifact.

```
design.kanvas
├── manifest.json          # schemaVersion, file meta, page index, checksums
├── tokens.json            # variableCollections, variables, themes, styles
├── pages/
│   ├── pg_01.page.json    # { page, nodes: Record<NodeId, Node> }
│   └── pg_02.page.json
├── components/
│   └── definitions.json   # componentDefinitions, libraries, publish history
├── prototype.json         # flows, interactions
├── assets/
│   ├── index.json         # asset metadata + hashes
│   └── blobs/<hash[0:2]>/<hash>.<ext>
└── history/
    ├── head.json          # revision, base snapshot ref
    └── events.ndjson      # optional export of the history log (for audit handoff)
```

```jsonc
// manifest.json
{
  "schemaVersion": 1,
  "file": { "id": "f_0192...", "name": "Mobile Login Exploration", "revision": 4821 },
  "pageOrder": ["pg_login", "pg_tokens", "pg_playground"],
  "createdAt": 1750000000000,
  "checksums": { "tokens.json": "sha256:…", "pages/pg_login.page.json": "sha256:…" },
  "exporter": { "app": "kanvas", "version": "0.1.0" }
}
```

Serialization rules:

- Numbers are emitted with at most 3 decimals; `-0` is normalized to `0`.
- `undefined` fields are omitted; empty arrays are omitted.
- Key order is canonical (sorted) so snapshots are diffable and hash-stable.
- Large files use **per-page lazy loading**: opening a file loads `manifest.json` + `tokens.json` +
  the current page; other pages load on demand (or on prototype navigation).
- Compression: blobs stored uncompressed, JSON as `gzip` inside the zip (deflate level 6).

### 17.1 Example: minimal document (login card fragment)

```jsonc
{
  "manifest": { "schemaVersion": 1, "file": { "id": "f_0192a", "revision": 12 } },
  "pages": [{
    "page": { "id": "pg_login", "name": "Login", "background": { "type": "SOLID", "color": { "r": 247, "g": 247, "b": 248, "a": 1 } } },
    "nodes": {
      "n_mobile": {
        "id": "n_mobile", "type": "FRAME", "name": "Login / Mobile", "pageId": "pg_login", "parentId": null,
        "transform": { "x": 0, "y": 0, "width": 390, "height": 844, "rotation": 0, "scaleX": 1, "scaleY": 1 },
        "clipsContent": true,
        "fills": [{ "type": "SOLID", "color": { "r": 255, "g": 255, "b": 255, "a": 1 }, "visible": true, "opacity": 1 }],
        "boundVariables": { "fills.0.color": { "type": "VARIABLE_ALIAS", "id": "v_surface_default" } },
        "children": ["n_card"], "opacity": 1, "blendMode": "NORMAL", "visible": true, "locked": false,
        "autoLayout": {
          "direction": "VERTICAL", "wrap": "NO_WRAP", "itemSpacing": 24,
          "padding": { "top": 48, "right": 24, "bottom": 48, "left": 24 },
          "primaryAxisAlign": "CENTER", "counterAxisAlign": "CENTER", "clipsContent": false,
          "strokesIncludedInLayout": false
        },
        "metadata": { "createdBy": "ag_claude", "createdAt": 1750000000000, "updatedBy": "ag_claude", "updatedAt": 1750000000000, "revision": 3 }
      },
      "n_card": {
        "id": "n_card", "type": "FRAME", "name": "Login Card", "pageId": "pg_login", "parentId": "n_mobile",
        "transform": { "x": 24, "y": 300, "width": 342, "height": 268, "rotation": 0, "scaleX": 1, "scaleY": 1 },
        "sizing": { "horizontal": "FILL", "vertical": "HUG" },
        "cornerRadius": 16,
        "effects": [{ "type": "DROP_SHADOW", "color": { "r": 0, "g": 0, "b": 0, "a": 0.08 }, "offset": { "x": 0, "y": 8 }, "radius": 24, "spread": 0, "blendMode": "NORMAL", "visible": true }],
        "autoLayout": { "direction": "VERTICAL", "wrap": "NO_WRAP", "itemSpacing": 16, "padding": { "top": 24, "right": 24, "bottom": 24, "left": 24 }, "primaryAxisAlign": "MIN", "counterAxisAlign": "MIN", "clipsContent": false, "strokesIncludedInLayout": false },
        "children": ["n_title", "n_email", "n_button"],
        "opacity": 1, "blendMode": "NORMAL", "visible": true, "locked": false,
        "metadata": { "createdBy": "ag_claude", "createdAt": 1750000000100, "updatedBy": "ag_claude", "updatedAt": 1750000000100, "revision": 2 }
      },
      "n_title": {
        "id": "n_title", "type": "TEXT", "name": "Title", "pageId": "pg_login", "parentId": "n_card",
        "transform": { "x": 24, "y": 24, "width": 294, "height": 28, "rotation": 0, "scaleX": 1, "scaleY": 1 },
        "sizing": { "horizontal": "FILL", "vertical": "HUG" },
        "characters": "Selamat datang kembali",
        "textStyle": { "fontFamily": "Inter", "fontStyle": "Semi Bold", "fontSize": 20, "lineHeight": { "unit": "PIXELS", "value": 28 }, "letterSpacing": { "unit": "PERCENT", "value": -1 }, "paragraphSpacing": 0, "textAlignHorizontal": "LEFT", "textAlignVertical": "TOP", "textCase": "ORIGINAL", "textDecoration": "NONE", "textAutoResize": "HEIGHT", "truncation": "DISABLED", "listStyle": "NONE", "listSpacing": 0, "hangingPunctuation": false, "hangingList": false, "fills": [{ "type": "SOLID", "color": { "r": 21, "g": 22, "b": 26, "a": 1 }, "visible": true, "opacity": 1 }] },
        "rangeStyles": [], "opacity": 1, "blendMode": "NORMAL", "visible": true, "locked": false,
        "metadata": { "createdBy": "ag_claude", "createdAt": 1750000000200, "updatedBy": "ag_claude", "updatedAt": 1750000000200, "revision": 1 }
      },
      "n_button": {
        "id": "n_button", "type": "INSTANCE", "name": "Button/Primary", "pageId": "pg_login", "parentId": "n_card",
        "transform": { "x": 24, "y": 180, "width": 294, "height": 48, "rotation": 0, "scaleX": 1, "scaleY": 1 },
        "sizing": { "horizontal": "FILL", "vertical": "FIXED" },
        "componentId": "n_button_master",
        "componentProperties": { "label": "Masuk", "state": "default", "showIcon": false },
        "overrides": [
          { "nodePath": ["label"], "property": "characters", "value": "Masuk", "mode": "SET", "authoredBy": "ag_claude", "authoredAt": 1750000000400 }
        ],
        "opacity": 1, "blendMode": "NORMAL", "visible": true, "locked": false,
        "metadata": { "createdBy": "ag_claude", "createdAt": 1750000000400, "updatedBy": "ag_claude", "updatedAt": 1750000000400, "revision": 1 }
      }
    }
  }]
}
```

Note how the agent-produced fragment carries **no geometry for laid-out children beyond their stored
values**: `x/y` are the last solved positions, and the autolayout solver recomputes them. This keeps
files small and makes layouts robust to text/font changes.

---

## 18. Validation, Migration, Compatibility

### 18.1 Validators (run on import, on agent command batches, and in CI fixtures)

| Validator | Checks | Failure behavior |
|---|---|---|
| `schema` | Shape against JSON Schema generated from the TS types | Reject import / reject command batch |
| `graph` | I1–I6 invariants, no cycles, no orphans | Reject; orphaned nodes are re-parented to page root with a warning |
| `references` | `componentId`, `assetId`, `styleRefs`, `boundVariables`, `variable.alias`, `interaction.destinationId` | Missing refs → dangling markers, never auto-delete |
| `variables` | Alias cycles, scope validity for the bound property, type match | Block the binding command with a typed error |
| `layout` | Autolayout children not `ABSOLUTE` with FILL, spacing ≥ 0, min ≤ max | Clamp with warning |
| `text` | Ranges sorted/non-overlapping, within `characters` bounds | Normalize silently |
| `limits` | ≤ 50k nodes/file, ≤ 2k nodes/frame, ≤ 40 variant values per axis | Reject with actionable message |

### 18.2 Migration

- `schemaVersion` is a single integer. Each migration is a pure function `(docVn) => docVn+1` in
  `packages/document/migrations/`, with a golden fixture (before/after) and a test.
- Reading an older file: run migrations in order, then persist lazily (never on open, only on first edit).
- Writing: always current version. A "compat export" toggle writes `schemaVersion-1` for older clients.
- Realtime protocol versions are negotiated in the WS handshake (`protocol: 3`); the server refuses
  sessions below `minSupportedProtocol` with a typed `UPGRADE_REQUIRED` error.

### 18.3 Extensibility

Third-party/plugin data lives only in `metadata.pluginData` (namespaced by plugin id) and in sidecar
entities keyed by node id. The core schema never gains plugin-specific fields; new core node types
require a schemaVersion bump, a migration, and a renderer implementation (documented in
`06-architecture.md` §12 "Extensibility & plugins").

---

## 19. Traceability to Requirements

| Requirement (product brief) | Where it lands |
|---|---|
| §6.1–6.2 Canvas & shapes | `03-feature-spec-core.md`, §3 `Node` union here |
| §6.3 Text | §8 here + `03-feature-spec-core.md` §6 |
| §6.4 Auto layout | §7 here + `03-feature-spec-core.md` §7 |
| §6.5 Components | §11 here + `04-feature-spec-systems.md` §2 |
| §6.6 Design system / tokens | §9 here + `04-feature-spec-systems.md` §3 |
| §6.7 Assets | §10 here + `04-feature-spec-systems.md` §4 |
| §6.8–6.9 Layers, pages, files | §4, §13 here + `03-feature-spec-core.md` §8–9 |
| §6.10 Prototyping | §12 here + `04-feature-spec-systems.md` §5 |
| §6.11 Comments & collaboration | §14 here + `04-feature-spec-systems.md` §6 |
| §6.12 Import/export | `08-export-and-codegen.md` |
| §7 AI agent layer | §15 here + `05-ai-agent-layer.md` |
| §9 Data model requirements | This whole document |
| §10 AI command model | §15.2 here + `05-ai-agent-layer.md` §7 |
| §12 Realtime expectations | §14 here + `07-state-model.md` §6 |
