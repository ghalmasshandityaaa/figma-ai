# 06 — System Architecture & Technical Design

> **Scope:** Deployment topology, monorepo structure, client rendering pipeline, document engine, realtime
> sync, backend schema, AI gateway, export workers, performance budgets, security, observability, environments,
> and ADRs.
> **Related:** `02-data-model.md`, `05-ai-agent-layer.md`, `07-state-model.md`, `08-export-and-codegen.md`,
> `09-implementation-plan.md`.

---

## 1. Architectural Principles

| # | Principle | Consequence |
|---|---|---|
| AP-1 | **Document-first, not UI-first** | Every mutation flows through `DesignCommand` → engine → CRDT → history. UI is a projection. |
| AP-2 | **Local-first latency** | Interactions apply optimistically on the client; the network confirms. No interaction waits on a round trip. |
| AP-3 | **Server-authoritative for permissions and locks** | The client never decides whether an agent may write; the gateway validates every tool call. |
| AP-4 | **Deterministic core** | Layout, token resolution, command application, and export are pure and reproducible. This is what makes agents reliable. |
| AP-5 | **Replaceable edges** | Sync (Liveblocks/Yjs), AI providers, and storage are behind interfaces. No vendor types leak into domain code. |
| AP-6 | **Compute where the data is** | Rendering and editing on the client; heavy export and model orchestration on servers/workers. |
| AP-7 | **One protocol, two brains** | Built-in and external agents use identical tool schemas and event envelopes. |
| AP-8 | **Fail visibly and recoverably** | Every subsystem has explicit error states and a recovery path; no silent data loss. |

---

## 2. System Topology

```
                         ┌──────────────────────┐
        users ──────────▶│  Vercel Edge / CDN   │  static assets, thumbnails
                         └──────────┬───────────┘
                                    │
                    ┌───────────────▼─────────────────────────────┐
                    │  Next.js app (Vercel)                        │
                    │  ├─ dashboard, auth callbacks                │
                    │  ├─ editor route (client-heavy)              │
                    │  ├─ present route (prototype player)         │
                    │  └─ REST route handlers (BFF)                │
                    └───┬─────────────┬──────────────┬────────────┘
                        │             │              │
             ┌──────────▼──┐  ┌───────▼───────┐  ┌───▼───────────────┐
             │ Supabase    │  │ Liveblocks     │  │ Agent Gateway     │
             │ Auth/Postgres│ │ (sync+presence)│  │ Node + uWS + Redis│
             │ Storage     │  │  or Yjs server │  │ tools, runs, locks│
             └─────────────┘  └───────┬────────┘  └───┬───────────┬───┘
                                      │                │           │
                                      │         ┌──────▼─────┐ ┌───▼──────────┐
                                      │         │ Claude API │ │ OpenAI API   │
                                      │         └────────────┘ └──────────────┘
                                      │                ▲
                                      │         ┌──────┴──────────┐
                                      │         │ MCP clients     │
                                      │         │ (Claude Code,   │
                                      │         │  Codex, custom) │
                                      │         └─────────────────┘
                          ┌───────────▼────────────┐
                          │ Export workers (Node)  │
                          │ Playwright/Chromium,   │
                          │ sharp, pdf-lib, queue  │
                          └───────────┬────────────┘
                                      │
                             Supabase Storage (artifacts)
```

Trust boundaries:

| Boundary | Controls |
|---|---|
| Browser ↔ Next.js BFF | Supabase JWT (httpOnly cookie), CSRF tokens for mutations, rate limits |
| Browser ↔ Liveblocks | Short-lived room token minted server-side, per-file room, read/write role |
| Browser ↔ Agent Gateway | Short-lived WS ticket (never long-lived token in URL), origin check, per-session scopes |
| Gateway ↔ Providers | Server-side keys (Vault), egress allowlist, per-tenant quotas |
| Export worker ↔ Storage | Service role scoped to job artifacts, signed URLs 15 min |
| MCP client ↔ Gateway | Bearer session token, file-scoped, scopes intersected with granting user |

---

## 3. Monorepo Structure

pnpm workspaces + Turborepo. TypeScript strict, shared ESLint/Prettier, Vitest, Playwright.

```
kanvas/
├── apps/
│   ├── web/                  Next.js 15 (App Router), Tailwind, React 19
│   │   ├── app/(dashboard)/  file browser, settings
│   │   ├── app/(editor)/f/[fileId]/page.tsx
│   │   ├── app/(present)/present/[fileId]/page.tsx
│   │   └── app/api/          BFF route handlers
│   ├── agent-gateway/        Fastify + uWebSockets.js + Redis + Postgres
│   ├── export-worker/        Node worker: PNG/PDF/SVG/code jobs (BullMQ)
│   ├── mcp/                  MCP server (stdio + streamable HTTP)
│   └── cli/                  `npx @kanvas/cli connect|login|doctor`
├── packages/
│   ├── protocol/             zod schemas + TS types: commands, events, tool contracts, WS envelopes
│   ├── document/             node ops, validators, history, migrations, snapshot codec
│   ├── layout/               auto-layout solver, geometry, text metrics adapter
│   ├── renderer/             Konva scene graph, node renderers, hit-testing, snapping
│   ├── editor/               React editor shell, panels, canvas integration, agent UI
│   ├── state/                Zustand slices, selectors, command dispatch client
│   ├── sync/                 SyncProvider + Liveblocks adapter + Yjs self-host adapter
│   ├── tokens/               resolution, lint, W3C/CSS/Tailwind export
│   ├── ai/                   intent, planner, context compiler, guardrails, playback planner
│   ├── export/               ExportPlan builder + emitters
│   ├── ui/                   Tailwind design system (Radix primitives)
│   └── testkit/              fixtures, golden files, Playwright helpers, deterministic clocks
├── supabase/                 migrations, RLS policies, seed, edge functions
├── docs/                     this specification set
└── turbo.json, pnpm-workspace.yaml, tsconfig.base.json
```

Dependency rule: `protocol` ← everything; `document`/`layout`/`tokens` are framework-free and run in Node,
browser, and workers; `renderer`/`editor` are browser-only; `ai` is isomorphic except provider SDK calls
(server-only entrypoints via `@kanvas/ai/server`).

---

## 4. Client Architecture

### 4.1 Layers inside the editor

```
EditorApp (React)
├── EditorProviders        error boundary, theme, toasts, i18n, feature flags
├── ShellLayout            top bar / rail / panels / prompt bar (CSS grid, resizable)
├── CanvasSurface
│   ├── CanvasViewport     wheel/pan/zoom, pointer capture, coordinate transforms
│   ├── KonvaStage         layers: static, content, overlay, agent
│   ├── DomOverlays        cursors, comments, toolbar, zoom, agent shelf
│   └── InputRouter        tool state machine (idle → drawing → dragging → editing)
├── Panels                 pages, layers, assets, components, design, prototype, agents
├── AgentSurfaces          prompt bar, plan card, timeline, approvals, shelf
└── CommandPalette, Dialogs, Toasts
```

### 4.2 Rendering pipeline

1. **State change** (local command, remote CRDT update, or layout result) marks nodes dirty in a
   `RenderInvalidationQueue` (Set of ids + reason).
2. **Frame scheduler** (rAF) computes:
   - visible canvas rect + zoom bucket;
   - top-level frames intersecting the viewport;
   - per-frame cache validity (`frameCacheVersion`).
3. **Layer strategy**:

| Layer | Contents | Update policy |
|---|---|---|
| `staticLayer` | Frames/shapes not currently edited, with `listening:false` | Cached with `Konva.Node.cache()` at zoom buckets (25/50/100/200/400%); re-cached on visual change |
| `contentLayer` | Text, instances, vectors, images, selected frame's subtree | Redrawn on invalidation; `perfectDrawEnabled:false` |
| `overlayLayer` | Selection, hover, handles, snap guides, measurements | Redrawn every pointer frame (cheap vectors) |
| `agentLayer` | Drag ghosts, change pulses, agent selection boxes | Redrawn on playback ticks (≤ 60fps) |

4. **Draw**: `layer.batchDraw()` per dirty layer. Never `stage.draw()` during interaction.

Frame caching rules:

- A frame is cacheable when it is not selected, not being dragged, and has no active animation.
- Cache key includes zoom bucket, theme id, and a content hash; invalidated by any command touching the frame.
- At zoom < 25%, a frame may be replaced by a pre-rendered thumbnail (from the asset pipeline) to keep
  thousands of frames cheap.
- Blur effects are expensive: frames with `LAYER_BLUR`/`BACKGROUND_BLUR` are excluded from caching only while
  resizing; shadows are cached with the frame.

### 4.3 Text and layout

- Text measurement uses `CanvasRenderingContext2D.measureText` with font fallback stacks; a
  `FontMetricsCache` keyed by `(family, style, size, letterSpacing)` stores ascent/descent/width.
- Layout solves run in a **worker** (`layout.worker.ts`) when a page exceeds 2,000 nodes or when a batch touches
  > 200 nodes. The worker returns `LayoutResult` patches; the main thread applies geometry without re-solving.
- During drag/resize of an autolayout container, a cheap "provisional solve" runs on the main thread (only the
  affected subtree); the full solve runs on drag end.
- Text editing uses a hidden `textarea`/contenteditable overlay positioned over the canvas for IME and
  accessibility, with the canvas rendering the styled result. Composition events are respected; the overlay is
  visible only as a caret and selection highlight drawn by the canvas.

### 4.4 Input & interaction

- Tool state machine: `IDLE → READY → DRAWING → DRAG_PREVIEW → COMMITTING → IDLE`, plus
  `TEXT_EDITING`, `BOX_SELECT`, `LASSO`, `PEN`, `COMMENT`.
- Pointer events are captured at the canvas root; tools receive normalized events in canvas coordinates.
- Drag previews mutate Konva nodes directly (no store writes, no history). On pointer-up, one `node.move` /
  `node.resize` command is dispatched with the final geometry.
- Keyboard shortcuts are resolved by a `CommandRegistry` (id, label, keys, when-clause, run). Panels, canvas,
  and agent surfaces register commands; conflicts are resolved by scope priority (canvas > panel > global).
- Undo/redo dispatches commands to the engine (not local-only), so collaborator views stay consistent.

### 4.5 Bundling & startup

- The editor route is dynamically imported; the dashboard bundle never includes Konva or the agent runtime.
- Konva, the agent UI, and the export dialog are separate chunks; the prompt bar is in the initial editor chunk
  (it is core UX).
- Target: editor TTI < 2.5s on a mid-tier laptop (M1/8GB) with a 200-frame file and warm cache; < 4s cold.
- Fonts: Inter + JetBrains Mono subsetted, `woff2`, preloaded; design-file fonts loaded on demand with
  `FontFace` and a fallback stack.

---

## 5. Document Engine

### 5.1 Command pipeline

```
dispatch(envelope)
  → schema validate (zod)
  → authorize (actor scopes, node permissions)
  → preconditions (revision, locks, reservations, node existence)
  → expand/compile (tools → commands; already compiled for UI)
  → dry-run validators (graph, refs, vars, layout)
  → apply to document view (immer draft)
  → compute inverse commands
  → layout solve (dirty subtrees)
  → persist to CRDT (Yjs transaction with origin=actor)
  → append HistoryEvent (Postgres async, ordered by revision)
  → broadcast presence/ack
  → emit audit event (if actor is agent or destructive)
```

Properties:

- **Transactional per envelope**: all commands apply or none. Validation happens before the CRDT write inside a
  single Yjs transaction; if the transaction aborts, no observer sees partial state.
- **Optimistic concurrency**: envelopes carry `baseRevision`; the engine uses per-node property versioning to
  detect conflicts (see `07-state-model.md` §6). CRDT convergence handles concurrent field writes; the engine
  adds semantic checks (e.g., a node cannot be its own ancestor).
- **Deterministic ids**: new node ids are generated client-side (UUIDv7) so replay/inverse is stable.
- **Inverse commands**: generated during apply using the `before` snapshot of touched fields. Verified by
  re-applying in tests.
- **History append**: async but ordered per file (a single writer per file, or revision assignment by the sync
  server). The history log is the audit source of truth, not the CRDT.
- **Batching**: max 50 commands per envelope; larger operations are split by the caller into progressive
  batches (with a batch id) so the UI stays responsive.

### 5.2 Document view & projection

The renderer never reads Yjs directly. A `DocumentView` projection materializes:

```ts
interface DocumentView {
  file: FileMeta;
  page: Page;
  nodes: Map<NodeId, Node>;
  order: Map<NodeId | 'root', NodeId[]>;      // paint order per parent
  resolved: ResolvedStyleIndex;                // token/style resolution for active theme
  layout: LayoutResult;
  selectionIndex: SpatialIndex;                // R-tree for hit-testing/query
  revision: number;
}
```

Projection updates are incremental: CRDT observers produce `NodePatch { id, changedKeys, prev, next }`, applied
to a mutable store (Zustand slice) with structural sharing for unchanged subtrees. Rebuilding the whole view is
only allowed on open, version restore, or schema migration.

### 5.3 Layout worker protocol

```ts
// main → worker
{ type: 'solve', jobId, roots: NodeId[], doc: SerializedDocSlice, themeId, scale: 1 }
// worker → main
{ type: 'solved', jobId, frames: Record<NodeId, Rect>, overflow: NodeId[], textMetrics: Record<string, Metrics> }
```

Timeout 250ms; on timeout, fall back to main-thread solve for the affected root and log a perf warning. Worker
is pooled (max 2) and restarted on crash; results are discarded if a newer job supersedes them.

---

## 6. Realtime & Sync Architecture

### 6.1 Provider abstraction

```ts
interface SyncProvider {
  connect(fileId: string, token: string): Promise<void>;
  disconnect(): void;
  getDoc(pageId: string): DocHandle;                 // CRDT handle
  transact(pageId: string, fn: (tx: Tx) => void, origin: string): void;
  onRemoteUpdate(cb: (pageId: string, origin: string) => void): Unsubscribe;
  awareness: AwarenessHandle;                        // presence, cursors, selection
  acquireReservation(req: ReservationRequest): Promise<Reservation | ReservationDenied>;
  releaseReservation(id: string): void;
  onReservationEvent(cb: (e: ReservationEvent) => void): Unsubscribe;
  status(): 'connected' | 'connecting' | 'offline' | 'error';
}
```

Adapters:

| Adapter | Use | Notes |
|---|---|---|
| **LiveblocksSyncProvider** (MVP) | Hosted realtime: Yjs doc sync, presence, comments storage, webhooks | Fastest path; comments can use Liveblocks threads or our own tables |
| **SelfHostedYjsProvider** (fallback) | Yjs + `y-websocket` + Postgres persistence + Redis pub/sub | Used if vendor lock-in/cost becomes an issue; API-identical |
| **LocalProvider** (tests) | In-memory, deterministic | Playwright and unit tests |

### 6.2 Document topology

- **File doc** (`Y.Doc`): file meta, page order, token collections/variables, styles, component definitions,
  libraries, flows, themes, asset index, comment thread anchors.
- **Page doc** (one `Y.Doc` per page, subdoc of the file doc): nodes map, per-parent order arrays, page
  background, page-local prototype interactions.
- Lazy loading: a page doc is loaded on open; other pages load when the user navigates or a prototype reaches
  them. Agents receive the current page only.
- Snapshotting: every 5 minutes and on idle (30s), the server persists a compacted snapshot (Yjs
  `encodeStateAsUpdate` → gzip) to Supabase Storage + a row in `file_snapshots` with `revision`. History events
  are separate and reference the snapshot for restore.
- Bootstrap: client fetches latest snapshot + subsequent updates from the sync server (Liveblocks handles this
  natively; self-hosted uses `y-leveldb`/Postgres as the update log).

### 6.3 Presence

- Presence payload: `PresenceState` minus verbose fields; ≤ 512 bytes; sent at 20Hz for cursor, event-driven for
  selection/tool.
- Clients interpolate remote cursors (linear interpolation over 80ms) and drop updates with `seq` ≤ last seen.
- Agent cursors use the same channel but carry `agent.status` and `cursor.state`; playback is local, so network
  presence is a low-frequency fallback (2Hz position snapshots) and receivers never animate agent cursors from
  network samples (they animate from local playback events). Cross-client consistency: all clients in the same
  room run the same playback script deterministically (seeded), so cursors match visually without high-frequency
  traffic. Non-participants see a 10Hz snapshot stream instead.
- Presence is never persisted; a 30s heartbeat marks stale actors offline.

### 6.4 Comments & notifications

- Comment threads stored in Postgres (durable, searchable) and indexed by file; their anchors (node id +
  canvas position) sync via the page doc (so node moves update pins).
- Mentions → `notifications` rows + optional email (Resend) + in-app inbox.
- Comment from an agent (reviewer) is a normal comment with `createdBy = ag_*`, rendered with the agent badge.

---

## 7. Backend: Supabase

### 7.1 Schema (core tables)

```sql
-- Identity & tenancy
users(id uuid pk, email text unique, display_name text, avatar_url text, created_at timestamptz)
workspaces(id uuid pk, name text, plan text, settings jsonb, created_at timestamptz)
workspace_members(workspace_id uuid, user_id uuid, role text, primary key(workspace_id, user_id))
-- Files & sharing
files(id text pk, workspace_id uuid, name text, owner_id uuid, schema_version int, revision bigint,
      thumbnail_key text, meta jsonb, deleted_at timestamptz, created_at timestamptz, updated_at timestamptz)
file_members(file_id text, user_id uuid, role text, primary key(file_id, user_id))
share_links(id uuid pk, file_id text, token text unique, role text, expires_at timestamptz, created_by uuid)
-- Content
pages(id text pk, file_id text, name text, index int, meta jsonb)
assets(id text pk, file_id text, kind text, name text, hash text, storage_key text, bytes bigint,
       width int, height int, meta jsonb, created_by uuid, created_at timestamptz)
file_snapshots(id uuid pk, file_id text, revision bigint, storage_key text, bytes bigint, created_at timestamptz)
versions(id uuid pk, file_id text, name text, description text, revision bigint, storage_key text,
         created_by text, agent_run_id text, auto boolean, created_at timestamptz)
history_events(id text pk, file_id text, page_id text, actor_id text, actor_type text, origin text,
               label text, batch_id text, envelope_id text, changed_node_ids text[], inverse jsonb,
               revision_before bigint, revision_after bigint, agent_run_id text, plan_step_id text,
               coalesce_key text, undoable boolean, ts timestamptz)
-- Collaboration
comment_threads(id text pk, file_id text, page_id text, anchor_node_id text, x double precision,
                y double precision, resolved boolean, resolved_by uuid, created_by uuid, created_at timestamptz)
comments(id text pk, thread_id text, body text, mentions uuid[], attachments text[], created_by text,
         created_at timestamptz, edited_at timestamptz)
-- Design system & libraries
libraries(id text pk, workspace_id uuid, name text, kind text, version text, published_at timestamptz)
library_versions(id uuid pk, library_id text, version text, changelog text, snapshot_key text,
                 published_by uuid, created_at timestamptz)
-- AI
agent_sessions(id text pk, file_id text, agent_id text, agent_name text, agent_kind text, created_by uuid,
               scopes text[], transport text, started_at timestamptz, ended_at timestamptz, last_seen timestamptz)
agent_runs(id text pk, file_id text, session_id text, agent_id text, prompt text, intent jsonb, plan jsonb,
           state text, summary jsonb, usage jsonb, parent_run_id text, started_at timestamptz, ended_at timestamptz)
agent_actions(id text pk, run_id text, step_id text, kind text, tool text, args jsonb, reasoning text,
              commands jsonb, result jsonb, status text, cursor_script jsonb, started_at timestamptz,
              ended_at timestamptz)
audit_log(id text pk, file_id text, actor_id text, actor_type text, kind text, summary text, payload jsonb,
          ts timestamptz default now())
-- Exports
export_jobs(id text pk, file_id text, requested_by uuid, scope jsonb, options jsonb, state text,
            progress int, artifact_key text, error text, created_at timestamptz, finished_at timestamptz)
```

Notes:

- IDs are text (prefixed ULIDs from the protocol), not UUIDs, to match the document model.
- `history_events` is append-only; retention 1 year (configurable). Large `inverse` payloads are fine (small
  command arrays); cold export to Storage for old files.
- `agent_runs`/`agent_actions` are retention-limited (default 90 days) and redactable on request; audit entries
  are retained longer and immutable.
- Row Level Security on everything; `service_role` only in gateway/workers.

### 7.2 RLS policy shapes

```sql
-- Read file if member, workspace member with access, or valid share link (checked at BFF)
create policy files_read on files for select using (
  exists (select 1 from file_members m where m.file_id = files.id and m.user_id = auth.uid())
  or exists (select 1 from workspace_members wm where wm.workspace_id = files.workspace_id and wm.user_id = auth.uid())
);
-- Writes require editor/owner
create policy files_write on files for update using (
  exists (select 1 from file_members m where m.file_id = files.id and m.user_id = auth.uid()
          and m.role in ('editor','owner'))
);
```

Agents never receive a Supabase JWT; the gateway uses `service_role` after validating the agent session token
and the granting user's current role. Every agent write is attributed in `history_events` and `audit_log`.

### 7.3 Storage buckets

| Bucket | Contents | Access |
|---|---|---|
| `assets` | Original uploads, content-addressed | Private, signed URL 15 min |
| `derived` | Thumbnails, optimized images, SVG rasterizations | Private, signed URL |
| `snapshots` | Compressed Yjs snapshots | Service only |
| `exports` | Generated ZIP/PDF/PNG artifacts | Private, signed URL 15 min, 24h TTL |
| `avatars` | User/agent avatars | Public-read |

Uploads: direct-to-storage signed upload URLs; the server validates MIME/size after upload, sanitizes SVG,
computes the hash, and only then registers the `assets` row. Orphan uploads expire in 24h.

---

## 8. AI Gateway (Agent Service)

### 8.1 Processes

```
HTTP (Fastify)                      WS (uWebSockets.js)
├─ POST /v1/sessions               ├─ /agent  (browser editor)
├─ GET  /v1/sessions/:id           ├─ /bridge (local CLI)
├─ POST /v1/runs/:id/pause|stop    └─ heartbeat + resume
├─ POST /v1/confirmations/:id
└─ POST /v1/mcp (streamable HTTP)
```

State in Redis: sessions, presence hints, reservations, idempotency cache (24h), rate limits, run dispatch
locks. Durable state in Postgres: runs, actions, confirmations, audit.

### 8.2 Tool server flow

```
receive toolCall
 → authenticate session (token) + verify run is EXECUTING
 → rate limit (per session/tenant)
 → schema validate (zod from protocol)
 → guardrail check (scopes, protected nodes, destructive gate, budgets)
 → reserve targets (if write)
 → compile to commands (pure, in @kanvas/ai/tools)
 → dry-run validators on current doc snapshot (fetched from browser? NO — see below)
 → dispatch commands envelope to the browser
 → await ack (revision + inverse) with timeout 5s
 → store result, release reservations, log audit
```

**Where does the authoritative document live for validation?** The browser holds the live doc, but the gateway
must not trust it for permissions or invariants. Solution:

- The **sync server** (Liveblocks/Yjs) is the authoritative CRDT. The gateway maintains a **read-only shadow
  projection** of the current page by subscribing to updates (server-side Yjs client, same room). It validates
  against that projection and includes the current revision in the envelope. The browser is the executor, but
  the command is rejected if its `baseRevision` is stale beyond a threshold or if validation fails server-side.
- For MVP cost reasons, a lighter variant is allowed: validate against the browser-provided compact snapshot
  **plus** server-side re-validation of the resulting envelope's inverse when it arrives. This is a documented
  risk (`R-04`) and must be replaced by the shadow projection before GA.

The tool result is sent to the model only after the browser acknowledges the commit (or the gateway times out
and marks the action failed). This is what makes the timeline truthful.

### 8.3 Concurrency & fairness

- One write run per file at a time; additional write runs queue (FIFO) with visible `WAIT` state.
- Read-only/audit runs may run concurrently (max 5).
- Redis locks: `file:{id}:writer` (TTL 20 min, renewed by run heartbeats), `res:{nodeId}` (TTL 60s).
- Run supervisor: if heartbeat is missed for 30s, pause the run, release reservations, notify clients.
- Model calls are streamed; the gateway can inject `run.pause` checks between tool calls (not mid-stream),
  keeping stop latency bounded by the current tool call.

### 8.4 Cost controls

- Per-tenant provider keys with quotas; per-run token/cost ceilings; cached system prompts and tool schemas
  (provider prompt caching); planner model vs executor model split; context pack budgets; hard stop at 100%.
- Anomaly alerts at 3× median cost for a prompt category.

---

## 9. Export Service

| Target | Where | Path |
|---|---|---|
| PNG/JPG/WebP (single frame, ≤ 4096px, ≤ 2×) | Client | Konva `stage.toBlob()` with a temporary export stage |
| PNG large/multi-frame/page | Worker | Chromium renders a minimal `/render` route with the same renderer package; tiling + `sharp` stitching |
| SVG | Client preferred, worker fallback | `@kanvas/export/svg` serializer over the ExportPlan |
| PDF | Worker | Chromium print or `pdf-lib` for vector pages; font subsetting |
| JSON / tokens | Client or BFF | Pure functions; deterministic |
| React/Tailwind/HTML/Flutter/RN | Client (small) or worker (large) | Pure emitters over ExportPlan; ZIP via `fflate` |
| ZIP packaging | Client for small, worker for large | Streaming ZIP in worker |

Job model: `export_jobs` table + BullMQ queue; progress via SSE/polling; artifacts to Storage with signed URL;
client-cancelable; idempotent by `hash(plan + options)` cached 24h. Full spec in `08-export-and-codegen.md`.

---

## 10. Key Sequences

### 10.1 Open a file

```
Browser → BFF: GET /api/files/:id  (auth cookie)
BFF → Supabase: file + membership + last snapshot metadata
BFF → Liveblocks: mint room token (role from membership)
Browser: load manifest + tokens (JSON) → open page doc (Liveblocks)
Browser: hydrate DocumentView, solve layout (worker if large), render static layer
Browser: subscribe presence, comments, agent events
```

Target: first frame visible < 1.5s warm, < 3s cold for a 5MB file.

### 10.2 Local edit

```
pointerup → dispatch(node.move)
  → validate/authorize/preconditions
  → apply to view (optimistic) → Konva update
  → Yjs transact (origin=user) → sync server broadcast
  → HistoryEvent append (async) → undo stack push
  → presence selection update
```

If the sync server rejects (permission change), the client reverts the optimistic apply and shows a toast.

### 10.3 Agent run

```
PromptBar → POST /v1/runs (gateway) with prompt + context refs
Gateway → planner (Claude/OpenAI) → Intent → Plan → broadcast plan
User approves (if required) → gateway → executor loop
  → toolCall → validate/guardrails → shadow-doc check → envelope → WS → browser
  → browser applies + acks → gateway stores action → model continues
  → playback orchestrator animates cursor locally on all clients
Gateway → finish → summary → audit + run report
```

### 10.4 Export

```
ExportDialog → POST /api/files/:id/export {scope, target, options}
BFF → create export_jobs row → enqueue BullMQ
Worker → build ExportPlan (fetch doc snapshot from sync server) → emit → package → Storage
Browser ← SSE progress ← BFF ← job updates
Browser → signed URL download (15 min)
```

---

## 11. Performance Architecture

### 11.1 Budgets

| Interaction | Budget (p95) | Mechanism |
|---|---|---|
| Pan/zoom frame | ≤ 16.7ms | Transform-only compositing; no re-layout |
| Drag node | ≤ 16.7ms | Imperative Konva preview; no store writes during drag |
| Selection change | ≤ 50ms | Spatial index + overlay redraw only |
| Property edit apply | ≤ 100ms | Local command + optimistic render; layout solve async |
| Text keystroke (canvas) | ≤ 16ms | Local text model, deferred command commit (300ms debounce) |
| Layer panel scroll | 60fps | Virtualization, 28px rows, memoized |
| File open (warm) | ≤ 1.5s | Snapshot cache, page lazy load, chunked bundles |
| Layout solve (5k nodes) | ≤ 2ms (worker) | Memoized intrinsic pass, dirty subtrees |
| Presence update render | ≤ 2ms/frame | Imperative cursor DOM updates, rAF |
| Agent playback overhead | ≤ 1.5ms/frame | DOM transforms, no React re-render |
| Sync propagation (same region) | ≤ 150ms p95 | Liveblocks/edge |
| Command batch apply (50 cmds) | ≤ 40ms | Single transaction, batched observers |

### 11.2 Scale targets

| Dimension | Target | Hard limit |
|---|---|---|
| Nodes per page | 5,000 comfortable | 20,000 (warning at 10k) |
| Frames per file | 500 | 2,000 |
| Pages per file | 100 | 500 |
| Assets per file | 1,000 | 5,000 |
| Concurrent editors per file | 20 | 50 |
| Concurrent agents per file | 2 write + 5 read | 10 total |
| File size (document JSON) | 10MB typical | 100MB |
| Undo history depth | 500 events | 2,000 |

### 11.3 Techniques

- Viewport culling + per-frame bitmap caches at zoom buckets.
- R-tree spatial index rebuilt incrementally for hit-testing and box selection.
- Layer list virtualization; inspector subscribes to narrow selectors.
- Image pipeline: `createImageBitmap`, thumbnails ≤ 512px, AVIF/WebP, `loading` budgets per viewport.
- Code splitting by route, panel, and dialog; prefetch the editor chunk on file-card hover.
- HTTP caching for immutable assets (hashed), `stale-while-revalidate` for thumbnails.
- Sync batching: presence at 20Hz max; document updates coalesced per animation frame on the sender.
- Long tasks: yield to the scheduler between command batches (`scheduler.yield()` fallback `setTimeout(0)`).

### 11.4 Degradation modes

| Condition | Behavior |
|---|---|
| Page > 10k nodes | Disable static-layer caching for offscreen frames beyond 2 viewports; show a "performance mode" toast |
| Device memory low / mobile | Reduce cache zoom buckets, disable trails, lower presence rate to 8Hz, disable thumbnails for far frames |
| Layout worker failure | Main-thread solve for visible roots; banner `Layout running in compatibility mode` |
| Sync latency > 1s | Show `Slow connection` in sync pill; keep local-first editing; increase debounce |
| WebGL/canvas unavailable | Read-only SVG snapshot + error page (no silent failure) |

---

## 12. Security, Privacy & Compliance

### 12.1 Threat model (top risks)

| Threat | Vector | Control |
|---|---|---|
| XSS via imported SVG/HTML | Malicious asset in canvas or comment | Server-side sanitize (allowlist tags/attrs), render SVG only through the vector parser, no `foreignObject`, CSP `script-src 'self'` |
| Comment XSS | Markdown with HTML | Markdown renderer with raw HTML disabled; link rel/attrs sanitized |
| SSRF via asset URL | Agent/image import with internal URL | Fetch proxy with DNS/IP allowlist (no RFC1918, no link-local), size/time limits |
| Token theft | Local bridge credentials | File mode 0600, short TTL, revocable, never in URLs/logs |
| CSRF on BFF | Cross-site form posts | SameSite=Lax cookies + double-submit CSRF token for mutations |
| Privilege escalation | Agent token scopes | Server-side intersection with current user role on every call; no client trust |
| Destructive agent action | Prompt injection / model error | Guardrails, confirmations, protected masters, budgets, audit |
| Data exfiltration | Agent network access | `allowNetwork=false` default, allowlisted asset hosts |
| Denial of service | Huge file/paste bomb | Node/size limits, upload caps (25MB/image, 50MB total), rate limits, streaming parse |
| Supply chain | npm dependency compromise | Lockfile, `pnpm audit` in CI, allowlisted postinstall, SBOM |
| Insider abuse | Admin misuse | Immutable audit, least privilege, workspace-level agent kill switch |

### 12.2 Headers & browser hardening

`Content-Security-Policy` with nonce-based scripts, `frame-ancestors 'none'` (embedding only via explicit
frame-ancestors allowlist for share links), `Strict-Transport-Security`, `Referrer-Policy:
strict-origin-when-cross-origin`, `X-Content-Type-Options: nosniff`, `Permissions-Policy` disabling camera/mic
(except explicit future features), COOP/COEP where needed for workers.

### 12.3 Data handling

- **Data at rest:** Postgres encryption at rest (Supabase), Storage SSE, secrets in Vault.
- **Data in transit:** TLS 1.3 everywhere; WSS for realtime.
- **Retention:** files until deleted (trash 30 days), history 1 year, agent logs 90 days, exports 24h, backups 30
  days PITR.
- **Deletion:** workspace/file deletion cascades with a 30-day restore window; hard-delete job clears Storage
  objects and snapshots; audit retains minimal tombstones (no content).
- **GDPR/CCPA:** export user data, delete account, DPA with sub-processors (Supabase, Vercel, Liveblocks,
  Anthropic, OpenAI), data residency option in V1 (EU region).
- **AI privacy:** prompts/context not used for training; provider zero-retention where available; PII redaction
  opt-in; no prompts in telemetry by default.

---

## 13. Observability & Reliability

### 13.1 SLOs

| SLO | Target | Measurement |
|---|---|---|
| Editor availability | 99.9% monthly | Synthetic + RUM session success |
| Command commit success | ≥ 99.95% | Engine acks vs attempts |
| Sync propagation | p95 ≤ 150ms same region | Client timestamps + server receive |
| Autosave durability | 0 committed commands lost | Snapshot/replay audits |
| Agent run reliability | ≤ 2% infra-caused failures | Gateway error taxonomy (excludes model errors) |
| Export success | ≥ 99% (valid scope) | Job outcomes |
| File open | p95 ≤ 3s cold | RUM |

### 13.2 Instrumentation

- **Traces:** OpenTelemetry across BFF, gateway, workers; trace ids propagated to the client via
  `traceparent` for run correlation. Spans: `command.apply`, `layout.solve`, `tool.call`, `provider.turn`,
  `export.render`.
- **Metrics:** Prometheus/Grafana or Supabase-compatible hosted (e.g., Axiom) for counters/histograms:
  run states, tool latency, playback lag, sync lag, cache hit rates, node counts.
- **Errors:** Sentry (client + server) with PII scrubbing and release tagging; source maps uploaded in CI.
- **Product analytics:** PostHog (events listed in `05` §13.1), feature flags, session replay for the dashboard
  only (never canvas content by default).
- **Audit:** append-only `audit_log`, mirrored to cold storage daily; optional hash chaining for tamper evidence
  (V1).

### 13.3 Failure handling

| Failure | User-visible behavior | Recovery |
|---|---|---|
| Sync server down | `Offline` pill; local edits continue (IndexedDB persistence) | Reconnect with exponential backoff; merge CRDT on return |
| Command rejected (permission) | Toast + revert of optimistic change | Re-fetch membership |
| Snapshot write fails | Warning banner after 3 failures | Retry queue; user can export to avoid loss |
| Agent gateway down | Agent cards show `Offline`, runs paused | Auto-reconnect 5 min; run can be resumed |
| Provider outage | Run `ERROR` with provider message, partial work kept | Retry with fallback model if configured |
| Export worker stuck | Job `FAILED` after timeout, retry button | Dead-letter queue + alert |
| Clock skew | Server timestamps authoritative for revisions | Client uses monotonic sequence for presence |

### 13.4 Backups & DR

- Postgres PITR 30 days; nightly logical dump to cold storage.
- Storage versioning + lifecycle rules.
- Snapshots every 5 min per active file (server-side), retaining 24h of rolling snapshots + named versions.
- Quarterly restore drill: restore a random file from snapshots and verify hash equality.
- RPO 5 min (active files), RTO 1 hour (editor read/write).

---

## 14. Environments & Delivery

| Env | Purpose | Data |
|---|---|---|
| Local | Docker compose: Postgres, Redis, MinIO (or Supabase CLI), mock Liveblocks provider | Seed fixtures |
| Preview | Per-PR Vercel deploy + branch database | Anonymized seed; isolated buckets |
| Staging | Release candidate, migrations rehearsal, load tests | Prod-like, synthetic |
| Production | Users | Real |

CI pipeline (GitHub Actions):

1. `typecheck`, `lint`, `unit` (Vitest), `schema` (protocol compatibility check).
2. `build` affected packages (Turborepo remote cache).
3. `e2e` smoke on preview (Playwright, 8 critical flows).
4. `a11y` axe audit on editor + player.
5. Migration check: `supabase db diff --linked` must be empty; apply expand/contract migrations in preview.
6. Release: Vercel promote + gateway/worker deploy (Fly.io/Render/Cloud Run) with health checks and canary 10%.

Versioning:

- **Protocol version** negotiated in WS handshake; support N and N-1 during rollout.
- **Schema version** for files; migrations run on read (lazy) and are idempotent.
- **Tool version** per MCP tool; additive changes within v1.
- **API version** for public endpoints (`/v1`).

---

## 15. Extensibility

- **Emitters** (export targets) implement the `Emitter` interface (`08-export-and-codegen.md` §8) and can be
  registered internally or by partners (signed plugin packages, server-side execution only).
- **Plugins** (V2): sandboxed iframe with a typed `postMessage` RPC; permissions declared in a manifest
  (`read:selection`, `write:nodes`, `export`, `ui:panel`); no direct DOM/canvas access; all mutations go through
  commands so they are undoable and attributed. Plugin execution is rate-limited and disabled in present mode.
- **Custom node types** are deliberately not supported until schema v2; plugins can compose existing primitives.
- **Agent tools** are data-driven (`ToolDefinition` registry); internal tools and MCP-exposed tools share the
  same registry. A partner can ship a tool package that compiles to existing commands only.

---

## 16. Architecture Decision Records (summary)

| ADR | Decision | Rationale | Consequences |
|---|---|---|---|
| ADR-01 | Konva over WebGL/custom renderer for MVP | Fast to ship, good 2D primitives, caching, hit-testing | Ceiling at very large scenes; mitigation: frame caches + culling; revisit at 20k nodes |
| ADR-02 | CRDT (Yjs) for live doc + command log for history | Convergence for concurrent edits; deterministic audit/undo | Two sources to reconcile; snapshots derived from CRDT; inverse commands validated |
| ADR-03 | Liveblocks first, self-host fallback | Realtime is not the differentiator; time-to-market | Vendor dependency; abstracted behind `SyncProvider` |
| ADR-04 | Client applies commands; gateway validates | Local-first latency; same path for humans/agents | Gateway needs a shadow projection for authoritative checks |
| ADR-05 | Zustand with slices, not Redux | Small, fast, selector-friendly; React-first | Discipline needed for selector granularity |
| ADR-06 | Tailwind + Radix for chrome | Speed + accessibility primitives | Design tokens must be enforced via CSS vars |
| ADR-07 | Supabase for auth/db/storage | One platform, RLS, fast start | RLS complexity; escape hatch to dedicated Postgres |
| ADR-08 | MCP as the external agent contract | Ecosystem alignment; Claude Code/Codex ready | MCP capability gaps; fallback REST/WS bridge |
| ADR-09 | Planner/executor model split | Cost/latency/quality balance | Two prompts to maintain; shared tool schema |
| ADR-10 | Gated playback for visible mode | Truthful "watch it happen" UX | Requires playback budget/compression; add fallback Fast mode |
| ADR-11 | Server-side export workers | Deterministic fonts, large files, PDFs | Infra cost; queue complexity |
| ADR-12 | No plugin execution in MVP | Security + focus | Extensibility via emitters/MCP only until V2 |
| ADR-13 | Handles for agent addressing | Token efficiency and fewer hallucinated ids | Handle lifecycle complexity; run-scoped mapping in gateway |
| ADR-14 | Next.js App Router with client-heavy editor route | Routing/marketing/auth + editor in one deploy | Bundle discipline; dynamic imports mandatory |

---

## 17. Requirement Traceability

| Requirement | Where satisfied |
|---|---|
| §14 Performance | §11 budgets/scale/degradation; `03` §13 |
| §15.1 Frontend stack | §3, §4 |
| §15.2 Konva canvas | §4.2, ADR-01 |
| §15.3 Zustand | §5, `07-state-model.md` §2 |
| §15.4 Liveblocks/Yjs | §6 |
| §15.5 Supabase | §7 |
| §15.6 AI provider layer | §8, `05` §12 |
| §15.7 Playwright | §14, `09` §7 |
| §16.1 Client layer | §4 |
| §16.2 Document engine | §5 |
| §16.3 AI agent layer | §8, `05` |
| §16.4 Collaboration layer | §6 |
| §16.5 Export layer | §9, `08` |
