# 07 — State Model, Commands, History & Conflict Resolution

> **Scope:** Client store topology, the design command catalog, undo/redo semantics (including per-agent undo),
> realtime state, optimistic updates, conflict resolution, playback state machine, persistence, time travel,
> and determinism hooks for testing.
> **Related:** `02-data-model.md` (entities), `05-ai-agent-layer.md` (agent lifecycle), `06-architecture.md`
> (sync/engine infrastructure).

---

## 1. Principles

1. **The CRDT is the live document; the command log is the truth about intent.** Every applied change has both a
   converged document state and an attributable event with an inverse.
2. **State is a projection, not a second source of truth.** Zustand holds a materialized view optimized for
   rendering and selectors. It is rebuilt on open and patched incrementally after.
3. **Undo is a command, not a state restore.** Inverses are dispatched through the same engine, validated, and
   attributed.
4. **Presence is ephemeral; history is durable.** Cursors/selections never enter the document or history.
5. **Humans win.** Human intent outranks agent intent, always, without a modal negotiation.
6. **Every state transition is observable.** Store actions emit typed events for telemetry and tests.

---

## 2. Client Store Topology

Zustand v5 with slice composition. One store instance per open file (`createEditorStore(fileId)`), provided
through React context. Cross-file state (file list, settings) lives in separate stores.

```
EditorStore
├── documentSlice     DocumentView projection: nodes, order, styles/variables resolved, revision
├── selectionSlice    selection ids, hover, focus, anchor, mode (replace/add/toggle)
├── viewportSlice     camera {x,y,zoom}, size, devicePixelRatio, panning, zooming
├── toolSlice         active tool, tool options (fill/stroke defaults), interaction state
├── uiSlice           panel sizes/tabs/collapse, dialogs, theme, density, focus mode
├── historySlice      local undo/redo stacks (event ids), history panel filters, pending batch
├── syncSlice         connection status, pending ops, latency, collaborators count
├── tokenSlice        resolved tokens for active theme (derived), token panel state
├── assetSlice        asset library cache, uploads in progress, search
├── prototypeSlice    flows, selected interaction, player runtime state (separate for present route)
├── agentSlice        agents, sessions, active run, plan, actions, confirmations, budgets
├── playbackSlice     cursor scripts, queue, clock, speed, reduced motion, lag metrics
├── promptSlice       draft prompt, attachments, history, queue, target agent
└── toastSlice        transient notifications
```

Rules:

| Rule | Detail |
|---|---|
| Selector granularity | Components subscribe to the narrowest selector; arrays/maps are reference-stable unless changed |
| No derived state duplication | Resolved tokens/layout are computed in selectors or memoized selectors, not stored twice |
| No document writes from components | Only command dispatch mutates the document; ephemeral previews use refs/Konva |
| Transient updates | Drag previews, cursor positions, and hover states bypass React (`ref` + `requestAnimationFrame`) |
| Persistence | Only `uiSlice`, `promptSlice` (history), and preferences persist to `localStorage`/IndexedDB; document persists via sync |
| Time travel | History events + snapshots, never replaying Zustand actions |

### 2.1 `documentSlice` shape

```ts
interface DocumentState {
  file: FileMeta;
  page: Page;
  nodes: Record<NodeId, Node>;
  order: Record<NodeId | 'root', NodeId[]>;
  revision: number;
  themeId: string;
  resolved: {
    fills: Record<string, Paint[]>;        // key: `${nodeId}:${path}`
    text: Record<NodeId, TextStyle>;
    layout: LayoutResult;
  };
  status: 'loading' | 'ready' | 'error';
  lastAppliedEnvelopeId?: string;
}
```

Patches arrive as `NodePatch[]`; the reducer applies them immutably with structural sharing. Patch application is
O(changed nodes), not O(document).

### 2.2 `selectionSlice`

```ts
interface SelectionState {
  ids: NodeId[];                 // ordered, primary last
  hoverId?: NodeId;
  anchorId?: NodeId;             // for shift ranges in layers
  mode: 'replace' | 'add' | 'subtract' | 'toggle';
  editingTextId?: NodeId;
  editingPathId?: NodeId;
  activeHandle?: HandleId;
  boxSelect?: { from: Vec2; to: Vec2; polygon?: Vec2[] };
}
```

Selection is local (not synced as document state) but broadcast via presence so collaborators see highlights.
Multi-user selection does not affect anyone else's ability to edit — it informs.

### 2.3 `agentSlice` shape

```ts
interface AgentState {
  agents: Record<string, Agent>;
  sessions: Record<string, AgentSession>;
  activeRunId?: string;
  runs: Record<string, AgentRun>;
  plans: Record<string, AgentPlan>;
  actions: Record<string, AgentAction>;
  confirmations: Record<string, ConfirmationRequest>;
  selectedAgentId?: string;
  timelineOpen: boolean;
  lastEventSeq: number;             // dedupe/resume
  effectivePolicy: GuardrailPolicy;
}
```

The client mirror is **event-sourced** from gateway events (`run.state`, `plan.updated`, `action.appended`,
`confirmation.requested`). It never computes authoritative state itself; it may optimistically mark an action as
`RUNNING` between request and event.

### 2.4 `playbackSlice`

```ts
interface PlaybackState {
  speed: 0.5 | 1 | 1.5 | 2 | 'instant';
  reducedMotion: boolean;
  cursors: Record<AgentId, AgentCursor>;
  queue: PlaybackJob[];              // per agent
  current?: PlaybackJob;
  lagMs: number;
  pausedAll: boolean;
}
```

---

## 3. The Command Model

### 3.1 Envelope

```ts
interface CommandEnvelope {
  id: string;                        // op_*
  fileId: string;
  pageId?: string;
  actorId: string;                   // u_* | ag_* | 'system'
  actorType: 'HUMAN' | 'AGENT' | 'SYSTEM';
  runId?: string;
  stepId?: string;
  baseRevision: number;
  clientSeq: number;                 // per session, for ordering/dedupe
  ts: number;
  label: string;                     // human-readable: "Move card", "Apply auto layout"
  intent?: string;                   // verbatim user/agent intent
  batchId?: string;                  // groups a run or a multi-step UI operation
  coalesceKey?: string;              // merges rapid same-actor edits
  undoPolicy: 'UNDOABLE' | 'MERGE_PREVIOUS' | 'NON_UNDOABLE';
  dryRun?: boolean;
  commands: DesignCommand[];
}

interface DesignCommand<T = JsonValue> {
  op: DesignOp;
  params: T;
}
```

Rules:

- Max 50 commands per envelope; larger operations are split (progressive batches with the same `batchId`).
- `baseRevision` is advisory for UI conflict hints; field-level versions decide conflicts (§6.2).
- `clientSeq` is monotonically increasing per session; the gateway/dedupe layer drops duplicates.
- `NON_UNDOABLE` is reserved for meta-operations (version restore bookkeeping, publish); agents cannot mark a
  design change non-undoable.
- `dryRun` returns `{ commands, diff, warnings }` without applying; used by previews and agent pre-checks.

### 3.2 Command catalog (authoritative names)

**(a) Nodes & structure**

| Op | Params | Inverse | Notes |
|---|---|---|---|
| `node.create` | `node` (full or partial + type), `parentId`, `index?` | `node.delete` (soft, removes only this subtree if created together) | Id generated by caller (UUIDv7). Children may be included for paste/duplicate. |
| `node.delete` | `nodeIds[]`, `mode: 'TRASH' \| 'PURGE'` | `node.restore` from trash snapshot | `TRASH` default; protected nodes rejected |
| `node.restore` | `nodeIds[]` | `node.delete` | Restores subtree + original parent/index when possible |
| `node.update` | `nodeId`, `patch` (whitelisted props) | previous values | Generic property patch; validators enforce types |
| `node.rename` | `nodeId`, `name` | previous name | Coalesces with typing |
| `node.setVisibility` | `nodeId`, `visible` | previous | |
| `node.setLocked` | `nodeId`, `locked` | previous | Humans can lock; agents cannot unlock human locks |
| `node.setPermissions` | `nodeId`, `permissions` | previous | Owner-only |
| `node.move` | `nodeIds[]`, `delta? \| positions?` | previous transforms | Coalesce `drag:<id>:transform` |
| `node.resize` | `nodeId`, `box`, `constraintsMode?` | previous transform | Re-solves children by constraints |
| `node.rotate` | `nodeId`, `rotation` | previous | |
| `node.reparent` | `nodeId`, `newParentId`, `index?` | previous parent/index | Cycle check; re-solves autolayout |
| `node.reorder` | `nodeId`, `index` | previous index | Same-parent ordering |
| `node.group` | `nodeIds[]`, `groupId` | `node.ungroup(groupId)` | Preserves absolute positions; new group bounds = union |
| `node.ungroup` | `nodeId` | `node.group` with saved child ids/bounds | Restores children to grandparent |
| `node.duplicate` | `nodeId(s)`, `offset`, `parentId?` | `node.delete` created ids | New ids for the whole subtree; overrides rewritten |
| `node.paste` | `parentId`, `payload` (subtree), `position?` | `node.delete` created ids | Internal clipboard format v1 |
| `node.setMask` | `nodeId`, `isMask` | previous | |
| `node.setConstraints` | `nodeId`, `constraints` | previous | |
| `node.boolean` | `operation`, `nodeIds[]`, `resultId` | `node.ungroup` (restore operands) | Non-destructive until flatten |
| `node.flatten` | `nodeId` | `node.restore` from snapshot | Destructive; confirmation |
| `node.outlineStroke` | `nodeId` | `node.restore` | |
| `node.align` | `nodeIds[]`, `axis`, `mode` | previous transforms | Pure geometry; no layout parent |
| `node.distribute` | `nodeIds[]`, `axis`, `spacing` | previous transforms | |
| `node.setDescription` | `nodeId`, `description` | previous | Agent rationale / handoff notes |
| `node.annotate` | `nodeId`, `annotation` | remove annotation | Agent pin metadata |

**(b) Styles**

| Op | Params | Inverse |
|---|---|---|
| `style.setFill` | `nodeId`, `paints` | previous paints |
| `style.setStroke` | `nodeId`, `strokes` | previous |
| `style.setEffects` | `nodeId`, `effects` | previous |
| `style.setOpacity` | `nodeId`, `opacity` | previous |
| `style.setBlendMode` | `nodeId`, `blendMode` | previous |
| `style.setCornerRadius` | `nodeId`, `radius`, `smoothing?` | previous |
| `style.createStyle` | `style` (id, kind, definition, name) | `style.deleteStyle` |
| `style.updateStyle` | `styleId`, `definition`/`name` | previous + affected nodes re-resolve |
| `style.applyStyle` | `nodeId`, `styleId`, `target` | previous + `styleRefs` |
| `style.detachStyle` | `nodeId`, `target` | previous |
| `style.deleteStyle` | `styleId` | restore style; bound nodes keep last resolved values |

**(c) Text**

| Op | Params | Inverse |
|---|---|---|
| `text.setContent` | `nodeId`, `characters`, `selectionBefore?` | previous characters + ranges |
| `text.setStyle` | `nodeId`, `style` (partial) | previous |
| `text.setRangeStyle` | `nodeId`, `start`, `end`, `style` | previous ranges (normalized) |
| `text.setAutoResize` | `nodeId`, `mode` | previous + previous box |
| `text.setListStyle` | `nodeId`, `listStyle`, `spacing` | previous |
| `text.setContentProperty` | `instanceId`, `propertyKey`, `value` | previous override |

**(d) Layout**

| Op | Params | Inverse |
|---|---|---|
| `layout.setAutoLayout` | `nodeId`, `config` | `layout.removeAutoLayout` + previous boxes |
| `layout.removeAutoLayout` | `nodeId`, `keepPositions` | `layout.setAutoLayout` + previous config |
| `layout.setSizing` | `nodeId`, `horizontal`, `vertical` | previous |
| `layout.setItemSpacing` | `nodeId`, `itemSpacing`, `counterSpacing?` | previous |
| `layout.setPadding` | `nodeId`, `padding` | previous |
| `layout.setAlignment` | `nodeId`, `primary`, `counter`, `alignContent?` | previous |
| `layout.setWrap` | `nodeId`, `wrap` | previous |
| `layout.setMinMax` | `nodeId`, `min/max` | previous |
| `layout.setAbsolute` | `nodeId`, `positioning` | previous (restores box) |
| `layout.setGrids` | `nodeId`, `grids` | previous |
| `layout.setChildOrder` | `nodeId`, `order[]` | previous order |
| `layout.reflow` | `nodeId` | previous boxes (derived) |

**(e) Components & instances**

| Op | Params | Inverse |
|---|---|---|
| `component.create` | `nodeIds[]`, `definition`, `componentNodeId` | `component.delete` (detach instances first? blocked if instances exist unless `force`) |
| `component.instantiate` | `componentId`, `parentId`, `box`, `properties?` | `node.delete` |
| `component.detach` | `instanceId` | `component.instantiate` (link restored; overrides preserved) |
| `component.swap` | `instanceId`, `componentId` | previous componentId + overrides snapshot |
| `component.setProperties` | `instanceId`, `values` | previous values |
| `component.setPropertyDefinitions` | `componentId`, `definitions` | previous definitions (instances reconciled) |
| `component.createVariant` | `componentSetId`, `values`, `node?` | `node.delete` variant + axis cleanup |
| `component.updateVariantAxes` | `componentSetId`, `axes` | previous + migration of existing variants |
| `component.publish` | `componentIds[]`, `semver`, `changelog` | unpublish (if no consumers) / version pointer revert |
| `component.delete` | `componentId`, `mode` | restore from definition + snapshot; instances flagged dangling |
| `component.setDescription` | `componentId`, `description` | previous |
| `instance.setOverride` | `instanceId`, `path`, `property`, `value` | previous override or REMOVE |
| `instance.resetOverrides` | `instanceId`, `paths?` | previous overrides |
| `instance.exposeProperty` | `instanceId`, `path`, `name` | hide property |
| `instance.setComponentProperty` | `instanceId`, `key`, `value` | previous |

**(f) Variables, themes, collections**

| Op | Params | Inverse |
|---|---|---|
| `collection.create` | `collection` | delete collection (+variables) |
| `collection.update` | `collectionId`, `patch` | previous |
| `collection.delete` | `collectionId`, `force` | restore collection + variables + bindings |
| `variable.create` | `variable` | `variable.delete` |
| `variable.update` | `variableId`, `patch` (name, scopes, description, codeSyntax) | previous |
| `variable.setValue` | `variableId`, `modeId`, `value` | previous value |
| `variable.alias` | `variableId`, `modeId`, `targetVariableId` | previous |
| `variable.delete` | `variableId`, `mode` | restore + rebind if captured |
| `variable.bind` | `nodeId`, `propertyPath`, `variableId` | `variable.unbind` + previous value |
| `variable.unbind` | `nodeId`, `propertyPath`, `materializeValue` | `variable.bind` |
| `variable.createMode` | `collectionId`, `name` | remove mode + values |
| `variable.updateMode` | `collectionId`, `modeId`, `name` | previous |
| `theme.create` | `theme` | `theme.delete` |
| `theme.update` | `themeId`, `patch` | previous |
| `theme.setMode` | `themeId`, `collectionId`, `modeId` | previous |
| `theme.setDefault` | `themeId` | previous default |
| `theme.delete` | `themeId` | restore (blocked if default or referenced) |

**(g) Assets**

| Op | Params | Inverse |
|---|---|---|
| `asset.register` | `asset` | `asset.delete` (if unused) |
| `asset.insert` | `assetId`, `parentId`, `box`, `mode` | `node.delete` |
| `asset.replace` | `assetId`, `newAssetId`, `scope: 'NODE' \| 'ALL'` | re-replace with previous + usage manifest |
| `asset.update` | `assetId`, `patch` | previous |
| `asset.delete` | `assetId`, `force` | restore + re-link usage |
| `asset.optimize` | `assetId`, `codec`, `quality` | delete derived variant |

**(h) Pages**

| Op | Params | Inverse |
|---|---|---|
| `page.create` | `page` | `page.delete` |
| `page.rename` | `pageId`, `name` | previous |
| `page.duplicate` | `pageId`, `name` | `page.delete` |
| `page.delete` | `pageId` | restore page + nodes (30 days) |
| `page.reorder` | `pageId`, `index` | previous |
| `page.update` | `pageId`, `patch` (background, meta) | previous |
| `page.moveNodes` | `nodeIds[]`, `targetPageId` | move back (with ids preserved) |

**(i) Prototyping**

| Op | Params | Inverse |
|---|---|---|
| `prototype.link` | `interaction` | `prototype.removeLink` |
| `prototype.updateLink` | `interactionId`, `patch` | previous |
| `prototype.removeLink` | `interactionId` | `prototype.link` |
| `prototype.setTransition` | `interactionId`, `transition` | previous |
| `prototype.setOverlay` | `interactionId`, `overlay` | previous |
| `prototype.setFlow` | `flow` | delete flow |
| `prototype.setHomeFrame` | `nodeId` | previous |
| `prototype.deleteFlow` | `flowId` | restore flow |

**(j) History, export settings, metadata**

| Op | Params | Inverse | Notes |
|---|---|---|---|
| `history.createVersion` | `name`, `description?` | delete version | Not part of design undo |
| `history.rollback` | `versionId` | restore pre-rollback state | Meta-op; creates a new revision |
| `export.configure` | `nodeId`, `settings[]` | previous | Node export presets |
| `meta.setDevStatus` | `nodeId`, `status` | previous | READY / IN_PROGRESS / NONE |
| `meta.setTags` | `nodeId`, `tags` | previous | |
| `meta.setTheme` | `fileId`, `themeId` | previous | Document-level theme preview |

**Explicitly not design commands** (separate services, no document undo): comment create/reply/resolve,
presence updates, agent run/session state, notification reads, viewport changes, selection changes, export job
execution. Comments have their own soft-delete and audit.

### 3.3 Validation order

1. Schema (`zod`) — type/format checks.
2. Authorization — actor scopes, node permissions, file role.
3. Structural — I1–I6 invariants (02 §4.1).
4. Reference integrity — assets, components, variables, styles, interaction destinations.
5. Semantic — e.g., min ≤ max, variant matrix rectangular, no autolayout cycle, font available.
6. Limits — node counts, command counts, payload size (≤ 1MB per envelope).
7. Apply in a single transaction; generate inverse; update layout; append history.

Rejections are atomic: no partial application. Error shape:

```ts
interface CommandError {
  code: 'SCHEMA'|'AUTH'|'INVARIANT'|'REFERENCE'|'SEMANTIC'|'LIMIT'|'CONFLICT';
  message: string;             // human-readable, actionable
  opIndex?: number;
  nodeId?: NodeId;
  details?: JsonValue;         // allowed values, current value, conflicting actor
  recoverable: boolean;
}
```

### 3.4 Worked example

```jsonc
{
  "id": "op_01J9Z8",
  "fileId": "f_0192",
  "pageId": "pg_login",
  "actorId": "ag_claude",
  "actorType": "AGENT",
  "runId": "r_01J9Z6",
  "stepId": "st_04",
  "baseRevision": 4820,
  "clientSeq": 17,
  "ts": 1750000012000,
  "label": "Insert email input",
  "intent": "Add an email field to the login card",
  "batchId": "batch_login",
  "coalesceKey": null,
  "undoPolicy": "UNDOABLE",
  "commands": [
    {
      "op": "component.instantiate",
      "params": {
        "componentId": "n_input_master",
        "parentId": "n_card",
        "box": { "x": 24, "y": 92, "width": 294, "height": 48 },
        "properties": { "label": "Email", "placeholder": "nama@email.com", "secure": false },
        "instanceId": "n_email"
      }
    },
    { "op": "layout.setSizing", "params": { "nodeId": "n_email", "horizontal": "FILL", "vertical": "FIXED" } }
  ]
}
```

Inverse (computed):

```jsonc
{
  "commands": [
    { "op": "layout.setSizing", "params": { "nodeId": "n_email", "horizontal": "FIXED", "vertical": "FIXED" } },
    { "op": "node.delete", "params": { "nodeIds": ["n_email"], "mode": "TRASH" } }
  ]
}
```

---

## 4. History & Undo

### 4.1 Event model

Each applied envelope produces exactly one `HistoryEvent` (02 §14) with:

- `actorId`, `actorType`, `origin`, `runId`, `stepId`, `batchId`
- `inverse: DesignCommand[]`
- `revisionBefore/After`
- `changedNodeIds`
- `coalesceKey` when the engine merged rapid changes (the previous event is absorbed: its inverse is extended,
  and only the latest event remains undoable)

Coalescing windows (per actor, per key):

| Key pattern | Window | Merge rule |
|---|---|---|
| `drag:<nodeId>:transform` | until drag end (explicit `commit`) | Keep the first `before`, latest `after` |
| `nudge:<nodeId>:transform` | 600ms | Same |
| `text:<nodeId>:content` | 800ms | Keep first before, latest after; ranges recomputed |
| `style:<nodeId>:<path>` | 500ms | Same |
| `padding:<nodeId>` | 500ms | Same |

Coalescing never crosses actors, run ids, or pages.

### 4.2 Undo stacks

- **Per-actor local stacks.** Each client keeps its own `undoStack`/`redoStack` of event ids for the local user
  only. `⌘Z` never undoes a collaborator's or an agent's work implicitly.
- **Agent runs** are undoable via explicit actions (`Undo run`, `Undo step`, `Undo from here`) and the History
  panel. This is a deliberate, documented behavior: an agent's 40 edits must not sit between a designer and
  their last manual change.
- When the local stack is empty and the last visible change was an agent's, `⌘Z` shows a hint toast:
  `Nothing of yours to undo · Undo Claude Code's run` (action button). It never silently reverts agent work.
- New local edits clear the redo stack. Remote edits do **not** clear local redo; on redo, precondition checks
  may drop conflicting commands (selective redo).

### 4.3 Inverse application algorithm

```
undo(eventId, mode = 'full' | 'selective'):
  event = history[eventId]
  if !event.undoable: reject
  inverse = event.inverse
  for each cmd in reverse(inverse):
      current = read(cmd.target)
      expected = event.after[cmd.target]
      if current != expected:
          if mode == 'selective':
              skip cmd; record skipped field
          else:
              if current is a *later* change by another actor:
                  downgrade to selective automatically
  dispatch({ actor = user, label = `Undo ${event.label}`, commands = appliedSubset,
             undoOf = event.id, undoPolicy = 'UNDOABLE' })
  if skipped.any: toast "Undid N of M changes; K were changed by others"
```

Selective undo is the default for conflicts; full undo only when nothing changed since.

Redo re-applies the original commands with the same precondition logic (store, per event, the `after` values).

### 4.4 Agent run undo

```ts
interface RunUndoRequest {
  runId: string;
  mode: 'ALL' | 'STEP' | 'FROM_STEP';
  stepId?: string;
}
```

- Collect all events with `runId` (and step filter), ordered by revision.
- Verify none have been modified since (per-field check); if some have, `ALL` downgrades to `BEST_EFFORT` and the
  confirmation shows exactly what will be kept: `3 of 12 changes were edited by you; they will be kept.`
- Apply inverses in reverse revision order in one envelope per event (not one giant envelope) to keep atomicity
  and history readable.
- Mark the run's actions as `REVERTED` (or `PARTIALLY_REVERTED`) with a link to the undo event.
- Restoration is itself undoable: `Undo revert` replays the original run's commands (subject to preconditions).

### 4.5 Version restore vs undo

| Action | Scope | Mechanism | History |
|---|---|---|---|
| Undo | One event (or run) | Inverse commands | Append-only, adds an undo event |
| Version restore | Whole file | Replace live CRDT state with a snapshot | Adds a `history.rollback` event + auto-snapshot |
| Trash restore | Deleted nodes | Restore from trash snapshot | Adds a restore event |

Version restore is destructive to the current state; it always creates a pre-restore auto-version named
`Before restore to <version>` so it is reversible.

### 4.6 Retention & limits

- 500 undoable events per client session (older events remain in History, just not in the local stack).
- 2,000 events or 90 days in the History panel (workspace-configurable).
- Coalesced events store only the net inverse; intermediate property states are not retained (by design).
- History events purge with file deletion; audit entries follow compliance retention.

---

## 5. Realtime State Model

### 5.1 Write path (local)

```
User action
  → command dispatch (validate locally, optimistic apply)
  → update DocumentView + layout (sync for small, async worker for large)
  → Yjs transact(origin = { actorId, sessionId, envelopeId })
  → sync provider broadcast
  → HistoryEvent append (queued, ordered by server revision)
  → undo stack push
  → telemetry event
```

The optimistic apply is authoritative locally until a rejection arrives (permission change, server-side
invariant). Rejection triggers the inverse and a toast; the optimistic update never lingers silently.

### 5.2 Read path (remote)

```
Yjs update (origin = remote actor)
  → observer extracts NodePatch[] (from Y.Map deltas or computed diff)
  → apply to DocumentView
  → re-solve layout for dirty roots
  → update selection validity (drop ids deleted remotely)
  → update presence highlights
  → if origin is an agent: ensure its cursor/playback is in sync (script is seeded by actionId)
```

Remote updates never touch the local undo stack.

### 5.3 Ordering & idempotency

- Document: CRDT convergence, no total order required.
- History: server assigns monotonically increasing `revision` per file; events are appended in that order.
- Tool calls: `idempotencyKey` (gateway), `clientSeq` (session), `envelopeId` (dedupe).
- Presence: `seq` per actor; drop stale samples.
- Agent events: `seq` per session + `lastEventSeq` on resume; cursor samples are excluded from replay.

### 5.4 Offline

- Yjs updates persist locally (IndexedDB `y-indexeddb` adapter) per file/page.
- Pending envelopes queue in `pendingOps` (IndexedDB), applied optimistically and flushed on reconnect.
- On reconnect: exchange Yjs state vectors, merge, flush pending; conflicts resolved per §6.
- Offline duration limit: 30 days local retention; UI warns after 24h offline that changes are local-only.
- Agents are disabled offline (they require the gateway); the prompt bar shows `Agent unavailable offline`.

---

## 6. Conflict Resolution

### 6.1 Policy summary

| Scenario | Policy | UX |
|---|---|---|
| Two humans edit different properties | CRDT merge | Silent |
| Two humans edit the same scalar property | Last-writer-wins per field (Yjs) | Toast on the loser's screen: `Maya changed the fill while you were editing — kept the latest. Undo?` |
| Two humans move the same node simultaneously | Detect at commit (node version changed since drag start) | Non-modal prompt: `Keep mine` / `Use theirs` (default keep mine) |
| Human edits while an agent holds a reservation | Human wins; reservation invalidated | Agent pauses, cursor withdraws, timeline records takeover |
| Two agents target the same subtree | Server reservation serializes | Second agent shows `WAIT` with holder name |
| Delete vs concurrent edit | Delete wins for the subtree; edits discarded | Toast with `Restore` (trash) + audit entry |
| Reparent under a deleted parent | Reparent to nearest living ancestor | Warning badge on node |
| Variable deleted while bound | Bindings become `DANGLING`; values materialized | Banner: `3 properties lost their token — rebind` |
| Library update vs local overrides | Three-way merge (base, local, new master) | Update preview lists conflicts per property |
| Version restore vs active edits | Restore wins; active edits captured in pre-restore version | Confirm dialog with diff |

### 6.2 Per-field versioning

Each node tracks property versions in a parallel CRDT map (not in the public node JSON):

```
nodeVersions: Y.Map<propertyPath, { rev: number, actorId: string, ts: number }>
```

On apply, every changed property path gets a new `rev` (Lamport-ish counter per file). This enables:

- Detecting "changed since I started dragging" without comparing values (fast).
- Selective undo (compare `rev` at read time).
- Conflict messages that name the actor and time.

Cost: ~24 bytes per property version; properties touched are few. Pruned by compaction when a node's versions
exceed 100 paths (keep the max rev per path).

### 6.3 Drag conflict protocol

```
dragStart(nodeIds): record {nodeId: versionAtStart}
pointerup: dispatch(node.move, expect: {nodeVersionEquals: versionAtStart})
  if precondition fails:
      compute remoteDelta = currentTransform - transformAtStart
      if |remoteDelta| < 0.5px: apply mine (noise)
      else: show "Maya moved this while you were dragging" [Keep mine] [Use theirs]
           default Keep mine after 5s (non-blocking; toast persists 10s)
```

Agents follow the same protocol, except a precondition failure causes an automatic yield (human wins) rather
than a prompt.

### 6.4 Reservation & takeover protocol

1. Agent requests reservation on subtree roots for a step (`EXCLUSIVE`, TTL ≤ 60s).
2. Server grants and broadcasts; clients render a subtle `agent editing` tint on the reserved subtree only while
   the agent's cursor is inside it.
3. Human pointer-down or keydown on a reserved node:
   - client immediately sends `reservation.invalidate { nodeId, actorId }`;
   - server revokes, notifies the agent;
   - agent's gateway cancels dispatch for that step, discards transient previews, parks the cursor;
   - run state → `PAUSED (takeover)`; timeline appends `Human takeover by Maya at step 4`.
4. Agent may resume only via explicit `Resume`; on resume it re-reads targets and re-plans the remaining steps.

Humans never see a blocking dialog for taking over their own file. Agents are guests.

### 6.5 Agent conflict events

When an agent's tool call fails with `CONFLICT`:

- The gateway attaches the conflicting actor + changed paths to the tool result.
- The executor may retry once with fresh geometry if the step is geometry-dependent; otherwise it re-plans.
- The timeline shows a `CONFLICT` row with a human-readable cause and a `Retry` action.
- No silent overwrite is ever attempted.

---

## 7. Playback State Machine (client)

```
IDLE ──enqueue──▶ QUEUED ──start──▶ MOVING ──arrive──▶ HOVER ──hold──▶ PRESS
  ▲                                                          │
  │                                                          ▼
  └──done── PULSE ◀──settle── COMMIT_WAIT ◀──dispatch── DRAG/DROP/TYPE
                            │
                            └──error/timeout──▶ ERROR ──recover──▶ IDLE
```

| State | Enter | Exit | Notes |
|---|---|---|---|
| `QUEUED` | action appended | scheduler picks | Max 3 queued beats; beyond that, compress |
| `MOVING` | keyframe t=0 | target reached (duration formula) | Interpolated position, 60fps |
| `HOVER` | arrival | hold elapsed (80–180ms) | Hover outline on target |
| `PRESS` | hold done | press duration (80ms) | Ring pulse |
| `COMMIT_WAIT` | command dispatched | ack or 5s timeout | UI shows a subtle `applying…` only if > 250ms |
| `SETTLE` | ack | 90ms | Drag preview becomes committed state |
| `PULSE` | settle done | 900ms | Changed-node highlight |
| `ERROR` | timeout/error | acknowledged | Cursor error glyph; log row |

Interruptions:

- `interrupt(user)` → graceful: finish current segment, park, mark remaining beats `SKIPPED` (not failed).
- `stop(run)` → discard queue, no further commits, cursor fades to standby after 1.2s.
- `speedChange` → applies to the next beat; current beat finishes at the new multiplier.
- `reducedMotion` → all beats use `FADE` (120ms) with 400ms holds, no trails.

Playback events are **locally derived** from the action stream: each `AgentAction` carries a
`CursorPlaybackScript` (or a template + params). All clients with the same script animate identically; only the
actor's own client is authoritative on timing, others follow broadcast `beat` events (for late joiners).

---

## 8. Persistence & Recovery

### 8.1 Local

| Key | Contents | TTL |
|---|---|---|
| `y-indexeddb/{fileId}/{pageId}` | CRDT updates | Until file deleted locally |
| `pendingOps/{fileId}` | Unsent envelopes | 30 days |
| `ui/{fileId}` | Panel sizes, active tab, zoom, last page | Indefinite |
| `prompts/{fileId}` | Last 50 prompts (local only) | 90 days |
| `playback` | Speed, cursor visibility | Indefinite |
| `assetCache/{hash}` | Decoded thumbnails | LRU, 200MB cap |

### 8.2 Server

- Yjs snapshot every 5 min + on idle (30s) per file; retained 24h rolling + daily for 30 days.
- Named versions (manual + pre-agent + pre-restore) retained indefinitely (workspace policy).
- History events retained 90 days (panel) / 1 year (audit).
- Recovery on crash: replay snapshot + updates from the sync server; if unavailable, local IndexedDB state is
  offered (`Restore unsynced changes`).

### 8.3 Startup sequence

```
load preferences → open file meta → open page doc (cache-first) → hydrate DocumentView
→ solve layout (worker if large) → render → subscribe sync/presence/comments
→ flush pending ops → verify revision → mark ready
```

If the local cache is newer than the server snapshot, the client shows a `Syncing local changes…` state and
merges (CRDT), never overwriting.

---

## 9. Time Travel & Diff

- `history.diff(fromRevision, toRevision)` computes a semantic diff: nodes added/removed/changed, with
  property-level before/after. Used by version history and agent run summaries.
- Diff is computed from snapshots when available, otherwise by replaying events into a scratch document view.
- Version restore uses a snapshot + `history.rollback`; the UI previews the diff before confirming.
- Per-action diff cards in the agent timeline use the same diff engine (bounded to changed nodes).

---

## 10. Determinism & Test Hooks

To make canvas and agent behavior testable:

- **Injected clock:** `Clock` interface (`now()`, `setTimeout`), real in production, fake in tests.
- **Seeded jitter:** playback jitter uses `hash(actionId)` → deterministic; tests can assert exact keyframes.
- **ID provider:** injectable UUIDv7 generator; fixtures use fixed ids.
- **E2E mode:** `?e2e=1` disables animations except a 0ms tick, sets playback speed to `instant`, and exposes:

```ts
window.__kanvas = {
  getState(): EditorSnapshot;                 // serializable
  dispatch(envelope): Promise<CommandResult>;
  waitForIdle(timeoutMs?): Promise<void>;
  getPlayback(): PlaybackState;
  getAgentRun(runId): AgentRun;
  select(ids): void;
  setViewport(v): void;
  openFile(fileId): Promise<void>;
};
```

- **Golden fixtures:** command sequences with expected document hashes, inverse replay equality, and layout
  results (see `09` §7).
- **Invariant checks in dev:** every apply runs graph + reference validators; failures throw in dev and are
  logged (never silently ignored) in production.

---

## 11. Acceptance Criteria

| # | Criterion |
|---|---|
| ST-1 | Undo of any event restores the exact prior values; inverse replay produces a document hash equal to the pre-event hash (property-based test over 1,000 random command sequences). |
| ST-2 | Remote edits never enter the local undo stack; `⌘Z` after a collaborator's edit undoes the local user's last change. |
| ST-3 | Undo run reverts all of a run's events in reverse order and leaves unrelated concurrent edits intact. |
| ST-4 | Selective undo reports the skipped count and names the actor who changed the property. |
| ST-5 | Two clients editing different properties converge to the same document hash within 1s. |
| ST-6 | A human edit on a reserved node pauses the agent in ≤ 1s and leaves no applied command from the interrupted step. |
| ST-7 | Deleting a frame while another user edits inside it discards the edits with a toast and an audit entry; the deleted frame is restorable from trash. |
| ST-8 | Optimistic edits rejected by the server are reverted within 500ms with a toast. |
| ST-9 | Offline edits survive reload and merge without data loss on reconnect. |
| ST-10 | Playback stop leaves no queued cursors or pending commits; all applied actions are in the timeline. |
| ST-11 | Version restore creates a pre-restore version and a `history.rollback` event, and is itself reversible. |
| ST-12 | Command envelope replay with the same id produces exactly one history event. |
| ST-13 | Layer selection after a remote delete drops stale ids without errors. |
| ST-14 | All command ops in §3.2 are covered by at least one unit test for apply + inverse. |

---

## 12. Open Questions

| # | Question | Default |
|---|---|---|
| Q1 | Should `⌘Z` optionally undo "all actors" like a shared history? | No; explicit History panel |
| Q2 | Keep local undo across page switches? | Yes, per file, 500 events |
| Q3 | Should agent run undo be offered after 10 minutes? | Yes via History; HUD affordance expires at 10 min |
| Q4 | Do we sync selection to the agent as context automatically? | Yes, selection is included; users can detach it |
| Q5 | Conflict toasts vs inline badges for remote property edits | Toast for 5s + persistent badge in inspect panel |
| Q6 | Retention for per-field version maps | Compact after 30 days or 100 paths |
