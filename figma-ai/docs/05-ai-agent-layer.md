# 05 — AI Agent Layer: Connection, Cursors, Planning & Execution

> **Scope:** Everything that makes agents first-class collaborators: transport and identity, runtime state,
> prompt → intent → plan, the tool contract, visible cursor choreography, execution modes, multi-agent
> coordination, guardrails, provider integration, and evaluation.
> **Depends on:** `02-data-model.md` (entities), `01-ux-architecture.md` (surfaces), `07-state-model.md`
> (command/history/lock mechanics).
> **Normative keywords:** MUST / MUST NOT / SHOULD.

---

## 1. Goals & Non-Goals

### 1.1 Goals

1. Any compatible agent (built-in, Claude Code, Codex, custom MCP client) can connect to a file in **under
   60 seconds** and appear as a visible cursor.
2. Agents operate through **typed tools** that compile to **atomic design commands** — the same commands the
   UI uses. No side-channel mutations.
3. Every run is **watchable**, **interruptible**, **explainable**, and **undoable as a unit**.
4. Agent work is **deterministic where it matters**: layout solves, token resolution, and command application
   produce identical results regardless of model choice.
5. Safety is structural: scopes, reservations, confirmations, protected nodes, budgets, audit.

### 1.2 Non-goals

- Letting agents execute arbitrary code, network requests, or shell commands inside the editor.
- Agents acting without a visible identity or without an audit trail.
- Autonomous long-running "background designers" in MVP (runs are user-initiated or explicitly delegated).
- Supporting every agent protocol on day one — MCP + two provider adapters is the MVP surface.

### 1.3 Design axioms

| Axiom | Consequence |
|---|---|
| **A1. The document is the only truth** | Agents never hold state the file cannot explain. All effects are commands. |
| **A2. Presentation is separate from mutation** | Cursor motion, drag previews, and typing previews are ephemeral; commits are atomic. |
| **A3. Humans outrank agents** | Any human interaction wins immediately; agents yield, then re-plan. |
| **A4. Bounded autonomy** | Every run has step, time, node, and cost budgets. |
| **A5. Explain decisions, not tokens** | One rationale per action, one intent per step; no chain-of-thought dumps. |
| **A6. Same tools, different brains** | The tool layer is model-agnostic; providers differ only in reasoning quality. |

---

## 2. System Overview

```
┌──────────────────────────── Browser (editor) ────────────────────────────┐
│                                                                          │
│  PromptBar ──▶ RunController ──▶ PlaybackOrchestrator ──▶ CursorLayer     │
│                    │                    │                                │
│                    │                    ├──▶ Transient previews (Konva)  │
│                    │                    └──▶ Change pulses               │
│                    ▼                                                     │
│              AgentClient (WS) ──▶ DocumentEngine.apply(envelope)          │
│                    │                    │                                │
│                    │                    └──▶ History/Audit/Presence       │
│                    ▼                                                     │
└────────────────────┼─────────────────────────────────────────────────────┘
                     │  WSS /agent  (JSON events, resumable)
┌────────────────────▼─────────────────────────────────────────────────────┐
│ Agent Gateway (server)                                                    │
│  ├── Session registry + auth/scopes                                       │
│  ├── Tool Server (validation, preconditions, idempotency, guardrails)     │
│  ├── Context Compiler (summaries, handles, token budgeting)               │
│  ├── Orchestrator (built-in agent loop) / External Agent Bridge           │
│  ├── Reservation & Conflict Manager                                       │
│  └── Audit + telemetry sink                                               │
└─────────┬───────────────────────────────┬─────────────────────────────────┘
          │                               │
   Claude API / OpenAI API          MCP clients (Claude Code, Codex, custom)
```

Two execution placements, one contract:

| Path | Where the "brain" runs | Transport to canvas | MVP |
|---|---|---|---|
| **Built-in agent** | Server orchestrator (our prompt + tools) | Server → WS → editor | Yes |
| **External agent** | User's CLI/agent runtime (Claude Code, Codex) | MCP/stdio → local bridge → WS, or remote MCP → WS | Yes |
| **Hosted custom agent** | Partner service using our API | HTTPS tool calls + WS event stream | V1 |

The gateway is intentionally thin: it validates, authorizes, compresses context, and relays. It does **not**
apply edits. Edits are applied by the client's Document Engine so they share undo, presence, and CRDT
semantics with human edits. In server-side batch jobs (exports, audits), the server applies commands through
the same engine on a headless document view.

---

## 3. Connection, Identity & Presence

### 3.1 Connect flows

**A. Built-in agent (default, one click)**

1. User clicks `Connect agent` → chooses *Built-in* → names it (default "Kanvas Agent"), picks a model tier
   (`fast` / `balanced` / `deep`).
2. Server mints an `AgentSession` with scopes derived from the user's permission on the file (editor → read,
   write; owner → + publish; admin → + destructive).
3. Editor creates the `Agent` identity, assigns the next free palette color, renders the standby cursor at the
   viewport's focal point (60% x, 55% y), state `STANDBY`.
4. Agent card appears in the Agents tab and the shelf.

**B. Claude Code via local bridge**

```bash
# user terminal
npx @kanvas/cli connect --file https://kanvas.app/f/f_01J…
# prints: Connected as "Claude Code". Session as_… expires in 12h.
# Add the MCP server to Claude Code:
claude mcp add kanvas -- npx @kanvas/mcp --session as_…
```

The CLI opens a WebSocket to the gateway and runs an MCP server over stdio for the local agent. Every MCP
tool call is forwarded to the gateway, which validates and relays to the browser. The browser only accepts
tool calls over an authenticated session; the local CLI never receives design write credentials directly.

**C. Codex / OpenAI-compatible**

- Either the same MCP path (Codex with MCP support), or an API key stored server-side with the built-in
  orchestrator pointed at an OpenAI model. Keys are encrypted at rest (Supabase Vault), never sent to the client.

**D. Custom MCP client**

Connect dialog shows a scoped MCP endpoint + bearer token:

```
https://mcp.kanvas.app/v1/{fileId}?session=as_…
Authorization: Bearer kv_mcp_…
```

Remote MCP is supported for tools that can reach the internet; the local bridge is recommended for latency
and for CLIs without remote MCP support.

### 3.2 Session token & scopes

```ts
interface AgentSessionToken {
  v: 1;
  sessionId: string;      // as_*
  agentId: string;        // ag_*
  fileId: string;
  scopes: PermissionScope[];         // read, write, destructive, publish, export, manage_tokens, manage_assets
  pages?: string[];                  // optional page restriction
  expiresAt: number;                 // default 12h, max 7d
  issuedBy: string;                  // u_*
  bindings: { ip?: string; client?: string };
}
```

Rules:

- Scopes are **intersected** with the granting user's effective permission at call time — a token never
  escalates. If the user's role drops to viewer, all writes fail immediately with `PERMISSION_REVOKED`.
- Tokens are session-bound and revocable from the Agents tab (`Disconnect` → immediate WS close + token revoke).
- Tokens never appear in URLs shared with other users; local bridge writes them to
  `~/.kanvas/credentials.json` with `0600`.
- A token grants access to one file only. Cross-file agent work requires a new session (V1: workspace-scoped
  sessions for design-system agents, explicitly granted).

### 3.3 Standby behavior

Connection MUST produce a visible cursor within 1s even before any prompt:

- Position: last cursor position for this agent+file if known, else viewport focal point.
- State `IDLE`, opacity 0.7, label at 0.7 opacity, 1.5px "breathing" drift (disabled under reduced motion).
- After 60s idle, dim to 0.5 but never hide; label remains readable.
- Standby cursors MUST NOT block clicks (`pointer-events: none` except the label tooltip).
- On page switch, the cursor moves to the new page at the same screen-relative position (with a 300ms fade
  transition rather than a cross-page fly-through).

### 3.4 Reconnect & session resumption

- WS reconnect uses exponential backoff (0.5s → 8s, jitter) for up to 5 minutes, then `OFFLINE`.
- In-flight run on disconnect → `PAUSED` with `reason: CONNECTION_LOST`; no commands are applied while
  disconnected; on reconnect, the HUD offers `Resume` (never auto-resume writes).
- Event replay: client sends `lastSeq`; server replays missed non-ephemeral events (state, plan, actions,
  confirmations). Cursor samples are **not** replayed (newer samples supersede).
- Idempotency: every tool call has `idempotencyKey`; replay of an already-applied call returns the stored
  result rather than re-applying.

---

## 4. Runtime State Machine

### 4.1 States

| State | Meaning | Cursor | Prompt bar |
|---|---|---|---|
| `OFFLINE` | No active session | Hidden (fades after 2s) | `Connect an agent` |
| `CONNECTING` | Handshake in progress | Ghost outline | Spinner in agent card |
| `STANDBY` | Connected, no active run | Idle, labeled | Enabled |
| `THINKING` | Parsing prompt / intent | Small orbit ring; label "Thinking" | Run HUD, Stop enabled |
| `PLANNING` | Producing/updating plan | Idle near selection, "Planning" | Plan card building |
| `WAITING_APPROVAL` | Plan or clarification pending | Idle beside plan card | `Review plan` CTA |
| `EXECUTING` | Applying planned steps | Active playback | Pause/Stop |
| `PAUSED` | User paused, takeover, or disconnect | Parked 24px from last target | Resume/Stop |
| `BLOCKED` | Needs confirmation/permission/input | Parked near blocking node | Reason + action |
| `ERROR` | Run failed unrecoverably | Error glyph near last target | Retry/Details |
| `COMPLETED` | Run finished | Victory settle, then idle | Summary + next prompt |

### 4.2 Transition table (authoritative)

| From | Event | To | Side effects |
|---|---|---|---|
| OFFLINE | `session.open` | CONNECTING | authenticate, assign color, load policies |
| CONNECTING | `session.ready` | STANDBY | render cursor, announce to presence |
| STANDBY | `run.start(prompt)` | THINKING | create AgentRun, auto-version if large, start budget timer |
| THINKING | `intent.ready` | PLANNING | persist intent, show interpreted summary |
| THINKING | `error` | ERROR | log, surface retry |
| PLANNING | `plan.ready` (approval required) | WAITING_APPROVAL | show plan card, notify user |
| PLANNING | `plan.ready` (no approval) | EXECUTING | start step 1, begin playback |
| WAITING_APPROVAL | `plan.approved` | EXECUTING | record approver + plan hash |
| WAITING_APPROVAL | `plan.rejected` / `run.cancel` | STANDBY | mark run `ABANDONED`, no changes |
| EXECUTING | `run.pause` | PAUSED | stop dispatch; finish in-flight atomic command |
| EXECUTING | `step.blocked` | BLOCKED | create confirmation request, park cursor |
| EXECUTING | `step.failed` (recoverable) | EXECUTING | retry per policy, annotate warnings |
| EXECUTING | `step.failed` (fatal) | ERROR | checkpoint remains applied, summary of partial |
| EXECUTING | `run.completed` | COMPLETED | summary, audit, cost, undo window (10 min) |
| EXECUTING | `human.takeover` | PAUSED | cancel reservation, record takeover |
| EXECUTING | `session.lost` | PAUSED | freeze dispatch, mark connection lost |
| BLOCKED | `confirmation.granted` | EXECUTING | bind grant to action hash, resume |
| BLOCKED | `confirmation.denied` / timeout | EXECUTING or PAUSED | skip step or pause per policy |
| PAUSED | `run.resume` | EXECUTING | re-validate reservations, resume at step boundary |
| PAUSED | `run.stop` | STANDBY | cancel remaining steps, keep applied changes |
| PAUSED | `run.stopAndRevert` | STANDBY | invert applied events, verify revision |
| COMPLETED | `run.acknowledged` | STANDBY | collapse HUD |
| ERROR | `run.retry` | THINKING | new run linked via `parentRunId`, same prompt |
| any | `session.close` | OFFLINE | persist last cursor, revoke token, stop run |

### 4.3 Invariants

- **INV-1:** No command may be applied for a session whose state is not `EXECUTING` (except user commands).
- **INV-2:** A command batch is atomic; pause/stop waits for the current batch only (≤ 1 batch, ≤ 50 commands).
- **INV-3:** Stop acknowledgement must be emitted within 250ms of receipt, and no new batch dispatched after.
- **INV-4:** State transitions are monotonic per run; terminal states (`COMPLETED`, `ERROR`, `ABANDONED`)
  never transition back (retry creates a new run).
- **INV-5:** Every state transition is persisted as an `AgentAction(kind='MESSAGE'|'PLAN'|…)` or a run
  event so the timeline can be reconstructed from the server alone.

### 4.4 Autonomy modes

| Mode | Behavior | When used |
|---|---|---|
| **Guided** (default) | Plan shown; approve only if large/destructive; confirmation per guardrails | All runs |
| **Supervised** | Approve every step | File policy `requirePlanApproval: ALWAYS`, regulated teams |
| **Watch** | Agent plans + executes, but writes are staged as suggestions until accepted | Reviewer agents, cautious users |
| **Fast** | No playback (pulses + timeline only), still fully logged | Power users, small viewports, reduced motion |
| **Read-only** | No writes at all; produces annotations/report | Audit/review agents, viewers |

Modes are per-run overrides of the file policy; the effective policy is the intersection. Watch mode stages
commands into a `SuggestedChangeSet` node overlay (green dashed) that the user accepts per node or wholesale.

---

## 5. Prompt → Intent → Plan

### 5.1 Pipeline

```
prompt (+ attachments + selection + page + tokens + components)
   │  normalize (locale, mentions, slash commands, attachments)
   ▼
IntentExtractor  ── structured output (JSON schema, temperature 0.1)
   │  if ambiguity confidence < 0.6 → at most 2 targeted questions, then defaults
   ▼
DesignIntent  (persisted on the run; shown in plan header as "What I understood")
   │
ContextPlanner ── selects relevant nodes/tokens/components/assets (budgeted)
   ▼
PlanComposer  ── steps with dependencies, tools, target handles, estimates
   │  policy: approval gate?
   ▼
Executor ── step-by-step tool calls + playback
```

### 5.2 Intent extraction rules

1. **Scope resolution.** "this", "selected", "card" → current selection if non-empty. "mobile login" with no
   selection → new mobile frame. If the prompt implies a new artifact but a selection exists, ask once:
   *"Add to the selected card or create a new frame?"* (default: new frame).
2. **Locale.** Respond in the prompt's language (Indonesian prompt → Indonesian messages). Plan step titles
   follow the same locale; node names may stay English for system compatibility (configurable).
3. **Design-system first.** If the file has components/tokens matching the requested roles, the plan MUST use
   them. If no system exists, the plan MUST use the starter library shipped with every file (Button, Input,
   Checkbox, Radio, Select, Card, Modal, Nav bar, Table row) and the default token set.
4. **Defaults** (when unspecified), in order of precedence: user's last-used preset → file default theme →
   product defaults.

| Aspect | Default |
|---|---|
| Mobile frame | 390 × 844 (iOS), safe area 44 top / 34 bottom |
| Desktop frame | 1440 × 900 |
| Density | comfortable: 8px grid, section gap 24–32, control height 48 |
| Type scale | 12 / 14 / 16 / 20 / 24 / 32, Inter |
| Radius | controls 10, cards 16, modals 20 |
| Color | file theme; if none, neutral light + single accent |
| Autolayout | always for composed screens and cards |
| Components | always for interactive elements |
| Prototyping | none unless asked |

5. **Forbidden interpretations.** The agent MUST NOT silently delete, re-theme, or restructure work outside
   the specified scope. "Make it premium" means visual refinement within selection, not content removal.

### 5.3 DesignIntent example (the Indonesian login prompt)

Prompt:
> tolong buatkan form login mobile, gunakan gaya modern minimalis, tambahkan email, password, tombol login,
> dan forgot password link. Pakai auto layout, spacing rapi, dan buat tampilan premium.

```json
{
  "version": 1,
  "requestId": "req_01J9Z6Q2",
  "locale": "id-ID",
  "rawPrompt": "tolong buatkan form login mobile, gunakan gaya modern minimalis, tambahkan email, password, tombol login, dan forgot password link. Pakai auto layout, spacing rapi, dan buat tampilan premium.",
  "summary": "Membuat layar login mobile minimalis dengan email, password, tombol login, dan tautan lupa password menggunakan auto layout.",
  "target": {
    "scope": "NEW_FRAME",
    "framePreset": "MOBILE",
    "frameSize": { "width": 390, "height": 844 },
    "pageId": "pg_login"
  },
  "theme": {
    "mood": ["modern", "minimal", "premium"],
    "density": "COMFORTABLE",
    "colorway": "LIGHT",
    "styleKeywords": ["clean", "calm", "generous whitespace", "subtle depth"]
  },
  "layout": {
    "pattern": "centered-form",
    "sections": ["brand", "heading", "fields", "primary-action", "secondary-action"],
    "responsive": false,
    "useAutoLayout": true
  },
  "elements": [
    { "role": "TITLE", "label": "Selamat datang kembali" },
    { "role": "SUBTITLE", "label": "Masuk untuk melanjutkan" },
    { "role": "INPUT", "label": "Email", "placeholder": "nama@email.com", "required": true, "componentHint": "Input/Default" },
    { "role": "PASSWORD", "label": "Password", "placeholder": "Masukkan password", "required": true, "componentHint": "Input/Password" },
    { "role": "LINK", "label": "Lupa password?" },
    { "role": "BUTTON", "label": "Masuk", "sizeHint": "LG", "componentHint": "Button/Primary" }
  ],
  "behavior": { "useTokens": true, "useComponents": true, "prototyping": "NONE", "accessibilityTarget": "AA", "motion": "NONE" },
  "constraints": { "maxSteps": 14, "protectedNodeIds": [], "budget": { "tokens": 60000, "usd": 0.5 } },
  "deliverable": { "exportAssets": [], "report": true },
  "ambiguities": []
}
```

### 5.4 Clarification policy

- Ask **at most 2** questions per run, only when the answer changes the plan materially (scope, destructive
  intent, missing brand/theme when two exist).
- Questions are asked in the prompt bar or as a cursor bubble with 2–4 option chips and a default. No modal.
- If the user does not answer within 20s, apply the default and state it: *"Saya pakai 390×844 (mobile) —
  beri tahu kalau mau ukuran lain."*
- Never ask about things that are inferable or have a documented default.

### 5.5 Plan composition rules

A plan is 3–12 steps for normal tasks; hard cap 20. Each step MUST:

- Express **one outcome** in user language ("Add email and password fields"), not a tool name.
- Have `intent` (why), `tools`, `targetNodeIds` (or planned targets), and dependencies.
- Be **independently skippable**: skipping step N must leave a coherent file (the composer declares
  `skipSafety: 'SAFE' | 'DEGRADED' | 'UNSAFE'`; unsafe steps force a re-plan when skipped).
- Be estimable (`estimateMs`) from historical medians by tool type.

Plans MUST include, when writing > 3 nodes:

1. A **structure** step (frames/containers + layout).
2. A **content** step (text/inputs/buttons).
3. A **styling/token** step (binds tokens; never inline hard-coded values when a token exists).
4. A **verification** step (overflow, contrast, missing fonts, token coverage).

Re-planning triggers: a tool call fails twice, a human takeover occurs, a target node disappears, or the
model's confidence in the next step < 0.5. Re-plans are recorded as `PlanRevision` entries in the timeline
(the user sees "Plan updated — reason: button master was locked").

---

## 6. Cursor & Motion System

This is a product surface, not an implementation detail. It must feel like a skilled human is working:
deliberate, smooth, never teleporting (except explicitly configured), never obscuring the result.

### 6.1 Cursor anatomy

```
        ╭─────────────────────────╮
        │ ✦ Claude Code · Adding  │   ← label pill: spark + name + optional status
        ╰──────────┬──────────────╯
                   │
                 ╭─╮
                 │ │                  ← arrow cursor, 18×24px, agent color fill,
                 ╰─╯                    white 1.5px stroke, drop shadow E1
        ╭──────────────────────────╮
        │ Using Input/Default for   │  ← tooltip bubble (optional), max 280px,
        │ consistency.              │    2 lines, agent color left border
        ╰──────────────────────────╯
```

| Property | Spec |
|---|---|
| Cursor glyph | Custom arrow SVG, 18×24, hotspot at tip (0,0), 1.5px white stroke + 1px rgba(0,0,0,.15) shadow |
| Label | 22px tall pill, radius 6, agent color background, white 12px/600 Inter text, max 26 chars + ellipsis, placed +14/+18 from hotspot, flips to left/above near viewport edges |
| Spark glyph | 10px 4-point star, white, before the name — the non-color identity marker |
| Tooltip | White (dark: `#1C1C22`) surface, 12px text, max 280px wide, 2 lines then ellipsis, 4s TTL, one at a time, queued |
| Drag ghost | 2px agent-color outline of the dragged node + 12% tint fill; no label on the ghost |
| Trail | 10 points over 320ms, 2px line at 20% opacity, fades; disabled in reduced motion |
| Press | 6px ring pulse at hotspot, 180ms |
| Selection box | 1px dashed agent-color rect + 8% tint while box-selecting |
| Change pulse | After a commit: 2px ring expands 8px→18px around changed node, 900ms, then 1.4s 8% tint fade |
| Z-order | Above canvas, below floating UI; multiple agents ordered by last activity; labels collision-avoided by 22px vertical offsets |
| Scale invariance | Cursor stays 18×24 screen px at all zoom levels; canvas-anchored position converts via `screen = origin + canvas * zoom`; labels counter-scale (`1/zoom` clamped 0.75–1.25) |

State visuals:

| State | Visual |
|---|---|
| `IDLE` | Cursor + name, 70% opacity, subtle breath |
| `MOVING` | Full opacity, trail on |
| `HOVER` | Target node gets 1px agent-color inset outline; cursor pauses |
| `PRESS` | Ring pulse; cursor dips 1px |
| `DRAG` | Ghost outline + trail; label reads `Moving`, `Resizing`, `Reordering` |
| `DROP` | Ghost eases into place 90ms, then change pulse |
| `TYPE` | Small caret glyph (I-beam) replaces arrow; label `Typing`; text previews at ≤30fps |
| `RESIZE` | Cursor becomes resize arrows for the duration |
| `WAIT` | Cursor + 3-dot ellipsis in label; no fake progress |
| `THINKING` | 3-dot orbit around label; cursor static |
| `ERROR` | Red 2px burst ring + label turns error red for 1.2s |
| `DONE` | Single green pulse + label `Done` for 1.5s |

### 6.2 Motion model

**Two layers of motion, never one:**

1. **Sample layer** (network): cursor positions arrive at 20–30Hz (`cursor.sample`), interpolated client-side.
2. **Script layer** (choreography): actions produce explicit keyframes with semantic timing; the local
   `PlaybackOrchestrator` drives the cursor through them, independent of network jitter.

At 60fps the cursor must move continuously; a dropped sample interpolates along the path rather than snapping.

**Timing formula** (screen-space distance so zoom doesn't distort perceived speed):

```
distance = hypot(dx, dy) * zoom                    // screen px
duration = clamp(200 + 0.35 * distance, 200, 750)  // ms
duration *= speedMultiplier                        // 0.5 | 1 | 1.5 | 2 | 0 (instant)
```

| Transition | Duration | Easing |
|---|---|---|
| Free move | formula | `cubic-bezier(.45, 0, .2, 1)` (accelerate 25%, decelerate 75%) |
| Micro move (< 24px) | 120ms | `cubic-bezier(.2, 0, 0, 1)` |
| Hover settle | 80ms | linear |
| Press down / up | 80 / 60ms | `ease-out` |
| Drag | `max(320, formula * 1.25)` | `cubic-bezier(.35, 0, .3, 1)` with 120ms accel, 160ms decel |
| Typing per char | 22ms + deterministic jitter (0–18ms) | linear; word pause 90ms, punctuation 160ms |
| Drop settle | 90ms | `ease-out` |
| Pulse | 900ms | `cubic-bezier(.16, 1, .3, 1)` |
| Instant mode | 0ms; cursor teleports with a 120ms label flash and "jump" marker | — |

Deterministic jitter: seeded by `hash(actionId)` so replays look identical; no `Math.random()` in scripts.

**Path shape.** For moves > 160px, insert a quadratic Bézier control point offset perpendicular to the
straight line by `min(60, max(8, 0.06 * distance))`, alternating side by action index. This avoids the
"laser-straight line" look. Do not overshoot targets (no spring bounce on arrival).

**Hover-then-act.** Every action includes a hover beat (100–180ms) before press. The cursor must be still for
at least 80ms before a press so humans can perceive the target. Exception: consecutive drags in a tight
sequence may share one hover.

**Drag choreography.**

```
approach (formula) → hover target handle (120ms) → press (80ms)
→ drag: 3-segment ease (accel 120ms, linear middle, decel 160ms)
→ pause 60ms at destination (shows the final position)
→ release (60ms) → commit → settle (90ms) → pulse changed node (900ms)
```

During dragging, the node's transform is a **transient preview** (Konva node updated imperatively), and the
document command is committed on release. If the drag is interrupted by a human, the preview is discarded and
no command is applied.

**Typing choreography.** The text node shows a client-side preview (caret + characters). The command commits
when typing completes or every 500ms for long strings (to keep undo granularity sane — max 5 commits per text
action). Long paragraphs (> 240 chars) switch to a 600ms fade-in of the full text with the label `Writing`.

**Playback compression.** Because execution is what matters, playback must never become a bottleneck:

- Each action has a `minPresentationMs` (default 220ms; 0 in Fast mode).
- If the model produces actions faster than playback, the orchestrator:
  1. raises the effective speed up to 2×,
  2. coalesces consecutive actions of the same kind on the same subtree into one beat ("Styling 6 layers"),
  3. drops trails and shortens hovers to 60ms,
  4. never drops the commit or the log entry.
- Playback lag is measured (`playback_lag_ms`); if > 1.5s sustained, the UI shows `Working quickly — see the
  full log` and switches to a summarized beat cadence.

**Interruptions.** Any user pointer/keyboard input intended for the canvas sets `interruptRequested`; the
orchestrator finishes the current 80ms segment and parks the cursor. It never aborts mid-drag without
discarding the preview (which it does atomically).

### 6.3 Playback script example

For `Create email input` (an instance insert at canvas (64, 420)):

```jsonc
{
  "id": "cur_01J9Z7A",
  "agentId": "ag_claude",
  "runId": "r_01J9Z6",
  "actionId": "act_01J9Z7",
  "totalDurationMs": 3120,
  "interruptible": true,
  "speedMultiplier": 1,
  "reducedMotionFallback": "FADE_LABEL",
  "keyframes": [
    { "t": 0,    "to": { "x": 980, "y": 300 }, "state": "MOVING",  "easing": "standard" },
    { "t": 620,  "to": { "x": 560, "y": 412 }, "state": "MOVING" },
    { "t": 940,  "to": { "x": 560, "y": 420 }, "state": "HOVER",   "holdMs": 140,
      "tooltip": { "text": "Inserting Input/Default", "kind": "TOOL" } },
    { "t": 1160, "to": { "x": 560, "y": 420 }, "state": "PRESS",   "press": { "at": { "x": 560, "y": 420 } } },
    { "t": 1300, "to": { "x": 560, "y": 420 }, "state": "DRAG" },
    { "t": 2480, "to": { "x": 300, "y": 420 }, "state": "DROP" },
    { "t": 2680, "to": { "x": 300, "y": 420 }, "state": "IDLE",   "holdMs": 120,
      "tooltip": { "text": "Fields use the same component for consistency", "kind": "MESSAGE" } }
  ],
  "resultPulse": { "nodeIds": ["n_email"], "kind": "CREATE", "durationMs": 900 }
}
```

### 6.4 Multi-agent cursor behavior

- Each agent owns one cursor; cursors never merge or hide each other.
- Labels are collision-avoided: if two labels would overlap, the later one offsets downward in 22px steps
  (max 3), then left.
- When two agents work near each other, a 40px exclusion radius keeps cursors from stacking; the orchestrator
  nudges the arriving cursor by 12–20px.
- An agent entering a page where another agent holds a reservation shows a `WAIT` state with tooltip
  `Waiting for Codex to finish this card` — this is important social legibility.
- Insane cursors (agents with no active run) are always dimmed so the active worker is visually dominant.

### 6.5 Accessibility & user controls

- `Agent speed`: 0.5× / 1× / 1.5× / 2× / Instant, per user; default 1×. Stored in settings.
- `Reduce motion` (OS or app) → `reducedMotionFallback`: cursor jumps to targets with a 120ms fade and a
  persistent 1.5s label; drag ghost becomes a position outline; trails disabled; pulses become fades.
- `Cursor visibility`: `Full` / `Minimal` (arrow + name only) / `Off` (agent still appears in the shelf and
  timeline; a subtle "agent editing" banner shows while commands apply). The Off mode MUST warn once that
  edits will still occur.
- Screen readers: announce agent state transitions at step boundaries via `role="status"` (e.g., "Claude Code
  started step 4 of 8: Add email and password fields"). Never announce cursor movement.
- Focus: users can press `⌥A` to open the Agents tab and `⌥.` to focus the active run's timeline.
- Contrast: label text is white on agent color; all palette colors meet 4.5:1 with white (verified, with dark
  text fallback for amber).

---

## 7. Tool Contract

### 7.1 Design of the tool layer

A tool is a **typed, validated, permissioned intent** that expands into 1..n design commands. Tools exist so
that models do not need to know the document schema or geometry math. The gateway compiles tool arguments into
commands, enforcing preconditions and guardrails.

```ts
interface ToolDefinition<A = unknown, R = unknown> {
  name: string;                          // "kanvas.create_frame"
  version: string;                       // "1.2.0", semver, additive changes only within v1
  summary: string;                       // one line, surfaced to the model
  description: string;                   // when to use / when not, 2–4 sentences max
  argsSchema: JsonSchema;
  resultSchema: JsonSchema;
  scopes: PermissionScope[];
  destructive: boolean;
  confirmation?: 'NEVER' | 'WHEN_LARGE' | 'ALWAYS';
  idempotent: boolean;
  preconditions?: ('PAGE_WRITABLE'|'NODE_UNLOCKED'|'NODE_NOT_PROTECTED'|'WITHIN_SCOPE'|'FONT_AVAILABLE')[];
  map(args: A, ctx: ToolContext): DesignCommand[];
  present?(args: A, result: R, ctx: ToolContext): CursorPlaybackScript | null;
}
```

Tool call envelope:

```ts
interface ToolCall {
  id: string;                    // tcall_*
  runId: string; stepId: string;
  tool: string; version: string;
  args: JsonValue;
  reasoning: string;             // required, one sentence, shown to humans
  confidence?: number;
  idempotencyKey: string;        // hash(runId+stepId+tool+args)
  dryRun?: boolean;              // returns commands + diff without applying
  expect?: {                     // optimistic preconditions
    revisionGte?: number;
    nodeExists?: string[];
    nodeTypes?: Record<string, NodeType>;
    propEquals?: Record<string, JsonValue>;
  };
  confirmationToken?: string;    // required when the action is gated
}
```

Results:

```ts
interface ToolResult {
  id: string;
  ok: boolean;
  changedNodeIds: NodeId[];
  handles?: Record<NodeId, string>;    // minted handles for created nodes
  revision?: number;
  data?: JsonValue;
  warnings: ToolWarning[];             // { code, message, nodeId? }
  error?: {
    code: 'VALIDATION'|'PRECONDITION'|'PERMISSION'|'CONFLICT'|'LIMIT'|'NOT_FOUND'|'INTERNAL';
    message: string;                   // actionable, model-readable
    details?: JsonValue;
    retryable: boolean;
  };
  durationMs: number;
}
```

Rules:

- **Validated before applied.** Schema validation, scope check, guardrails, then compile to commands, then the
  engine's own validators. A tool never partially applies (all commands in one envelope).
- **Idempotent by key.** Duplicate calls return the cached result.
- **Dry run supported** for all write tools; used by the plan preview and by agents that want to check before
  committing.
- **Bounded output.** Results return handles and diff summaries, never full document JSON (the model calls
  `get_node` when it needs more).
- **Errors teach.** Error messages include the offending field, allowed values, and a suggested fix.
  (`"minWidth 320 exceeds maxWidth 240 on n_card. Set maxWidth ≥ 320 or lower minWidth."`)

### 7.2 Tool catalog (v1)

Read tools (scope `read`, never destructive, no confirmation):

| Tool | Purpose | Returns |
|---|---|---|
| `kanvas.get_context` | Page summary, selection, theme, token/component inventory, recent changes | Compact tree + counts |
| `kanvas.get_selection` | Current selection details | `CompactNode[]` + handles |
| `kanvas.query_nodes` | Selector-based search (type, name, inFrame, hasAutolayout…) | `CompactNode[]` (≤ 200) |
| `kanvas.get_node` | Full node JSON at depth (default 2) | Node subtree |
| `kanvas.list_tokens` | Variables with values per active theme, grouped by category | Token list (~400 max) |
| `kanvas.list_components` | Available components/variants with property definitions | Compact definitions |
| `kanvas.search_assets` | Library search by name/tag/kind | Asset refs + dimensions |
| `kanvas.measure` | Text metrics, gaps, contrast between two nodes | Numbers + ratios |
| `kanvas.audit_accessibility` | Contrast, tap targets, alt text, heading order, focus order | Findings with node refs |
| `kanvas.audit_layout` | Overflow, off-grid spacing, inconsistent radii/typography | Findings |
| `kanvas.diff_since` | Changes since a revision/run | Summary + node ids |

Write tools (scope `write`):

| Tool | Key args | Compiles to | Presentation |
|---|---|---|---|
| `kanvas.create_frame` | `parent?, box, name, preset?, autoLayout?, fill?, radius?` | `node.create` (+`layout.setAutoLayout`) | Drag-frame choreography |
| `kanvas.create_shape` | `type, box, name, style?` | `node.create` | Draw choreography |
| `kanvas.create_text` | `parent, box?, characters, style, autoLayoutSizing?` | `node.create` (+`text.setStyle`) | Click → type |
| `kanvas.apply_auto_layout` | `nodeIds[], direction, gap, padding, align, sizing?` | `layout.setAutoLayout`, `layout.setSizing` | Hover → press layout button → pulse |
| `kanvas.set_layout` | `nodeId, patch` (spacing/padding/align/wrap/min/max) | `layout.*` | Panel-hover beat + pulse |
| `kanvas.transform_node` | `nodeId, move?, resize?, rotate?` | `node.move`, `node.resize`, `node.rotate` | Drag/resize choreography |
| `kanvas.reparent_node` | `nodeId, newParentId, index?` | `node.reparent`, `node.reorder` | Drag with insertion indicator |
| `kanvas.reorder_node` | `nodeId, index` | `node.reorder` | Layer-row drag |
| `kanvas.group_nodes` / `kanvas.ungroup` | `nodeIds[]` / `nodeId` | `node.group` / `node.ungroup` | Selection box → shortcut pulse |
| `kanvas.duplicate_node` | `nodeId, offset?` | `node.duplicate` | Alt-drag choreography |
| `kanvas.set_style` | `nodeId, fills?, strokes?, effects?, radius?, opacity?, blend?` | `style.*` | Panel beat + pulse |
| `kanvas.set_text` | `nodeId, characters?, ranges?, style?` | `text.setContent`, `text.setStyle`, `text.setRangeStyle` | Caret typing |
| `kanvas.bind_token` | `nodeId, propertyPath, variableId \| createFromValue` | `variable.bind` (+`variable.create`) | Token picker beat |
| `kanvas.create_variable` | `name, type, value, collection?, category, scopes` | `variable.create` | Token panel beat |
| `kanvas.create_component` | `nodeIds[], name, properties?` | `component.create` | Selection → ⌘⌥K pulse |
| `kanvas.insert_instance` | `componentKey, parent, box, properties?, overrides?` | `component.instantiate` | Asset drag |
| `kanvas.set_instance_properties` | `instanceId, props, overrides` | `component.setProperties`, `instance.setOverride` | Panel beat |
| `kanvas.create_variant` | `componentSetId, axisValues, baseComponentId?` | `component.createVariant` | Variant panel beat |
| `kanvas.insert_asset` | `assetId, parent, box, scaleMode?` | `asset.insert` | Drag from asset panel |
| `kanvas.link_prototype` | `sourceId, destinationId, trigger, actions, transition?` | `prototype.link` | Connector drag |
| `kanvas.create_version` | `label` | `history.createVersion` | Timeline pulse |
| `kanvas.annotate` | `nodeId, kind, text` | `meta.annotate` | Pin drop |

Destructive / gated tools (scope `destructive` unless noted):

| Tool | Gate |
|---|---|
| `kanvas.delete_node` | `WHEN_LARGE` (> 1 node or has children) → confirmation with diff |
| `kanvas.detach_instance` | Confirmation (breaks link) |
| `kanvas.delete_variable` / `kanvas.delete_component` | `ALWAYS` + usage report; blocked if protected |
| `kanvas.migrate_tokens` | `ALWAYS` + dry-run diff |
| `kanvas.publish_library` | Scope `publish` + `ALWAYS` |
| `kanvas.rollback_version` | `ALWAYS` + diff + auto-snapshot |
| `kanvas.export_code` | Scope `export`; no confirmation, but cost/time estimate shown for large files |
| `kanvas.ask_user` | Not destructive; pauses run until answered/timeout |

Meta tools:

| Tool | Purpose |
|---|---|
| `kanvas.propose_plan` | Submit/replace the plan for approval (used when the model re-plans mid-run) |
| `kanvas.ask_user` | Ask ≤ 2 questions with options + defaults; creates a `WAITING_APPROVAL`/`BLOCKED` state |
| `kanvas.report_progress` | Short human-readable status for the run HUD (rate-limited to 1/2s) |
| `kanvas.finish` | Submit summary, warnings, suggestions; terminal action of a run |

Tool naming and grouping MUST stay stable. Adding a tool is a minor version; changing args is a major version;
the gateway exposes only the latest version to new sessions and pins the version for the life of a run.

### 7.3 Context compiler

Models fail on context volume, not capability. The compiler produces a **budgeted context pack** per step:

```ts
interface ContextPack {
  fileSummary: { name, pageCount, nodeCount, theme, density, locale };
  page: { id, name, frameCount, viewport };
  selection: CompactNode[];             // ≤ 30
  targets: CompactNode[];               // step targets, full-ish
  siblings: CompactNode[];              // ≤ 20, for alignment decisions
  tokens: { byCategory: Record<string, {name, value, aliasOf?}[]> };  // ≤ 300
  components: CompactComponent[];       // ≤ 60
  assets: CompactAsset[];               // ≤ 20
  recentChanges: CompactHistoryEvent[]; // ≤ 10
  constraints: GuardrailPolicy;
  budget: { tokens: number; usedTokens: number };
}
```

Budgeting rules (per step, default model context 200k):

| Bucket | Budget | Overflow behavior |
|---|---|---|
| System + tool schemas | ~8k | Fixed |
| Tokens/components inventory | ~6k | Prioritize those matching roles in the plan; summarize the rest by count |
| Node context | ~12k | `get_node` on demand; sibling summaries truncated to 60 chars |
| Conversation history | ~10k | Rolling window; older tool results replaced by one-line outcomes |
| Screenshots | ~1.5k tokens each, max 2 | Only when visual judgment is needed; downscale to 1024px longest edge |
| Headroom | ≥ 20% | Reserved for tool results |

Compression techniques: run-scoped handles, `CompactNode` rows, diff summaries instead of state dumps,
"unchanged since last step" markers, and a per-run **memo** of computed answers (font metrics, contrast
pairs) cached by the gateway.

Images: screenshots are captured on demand (viewport or frame), never automatically each step; the model must
request `kanvas.get_screenshot` (V1) or rely on structural context. This keeps cost predictable and avoids
shipping user content unnecessarily.

### 7.4 Prompting strategy (built-in agent)

System prompt sections (in order, each compact):

1. **Role & product model:** you edit a design document through tools; never invent node ids; prefer tokens and
   components; one action at a time.
2. **Hard rules:** don't delete without confirmation; don't edit protected masters; don't exceed scope; use
   autolayout for composed screens; bind tokens; ask at most two questions; respond in the user's locale.
3. **Design heuristics:** spacing scale, type scale, hierarchy, contrast, tap targets, alignment, density,
   platform conventions. (A short, opinionated guide, ~500 tokens — quality of output depends on it.)
4. **Tool usage contract:** call `get_context` first if the page is unknown; after each write, check the result;
   don't blind-retry; use `dry_run` for risky restructures.
5. **Planning format:** JSON plan via `propose_plan`; steps must be outcomes, not tools.
6. **Style/verbosity:** rationale ≤ 1 sentence per action; no internal deliberation in messages; user-facing
   messages are short, specific, and honest about failures.

Two model roles:

- **Planner** (higher quality, low temperature): intent + plan + step sequencing. Called once per run + on
  re-plan.
- **Executor** (fast, tool-use optimized): one tool call per step iteration, with tight context. May batch
  independent tool calls (e.g., create three sibling texts) in one turn when the plan says so.

This split reduces cost and latency and makes plans stable while execution stays responsive. Both roles use
the same tool schemas.

---

## 8. Execution Model

### 8.1 Three modes

| Mode | Commands | Playback | Use |
|---|---|---|---|
| **Direct** | Applied as soon as the tool returns | Pulses + timeline only | Headless/export jobs, power users, reduced motion |
| **Visible** | Gated to playback beats; commits at press/drop | Full choreography | Default |
| **Hybrid** | Structure/content commits gated; styling batches applied immediately with a "styling pass" beat | Compressed choreography | Large runs, to avoid 10-minute choreography |

Default = **Visible** for runs ≤ 25 actions, **Hybrid** above that; both are user-overridable. The mode never
changes what is logged.

### 8.2 Step loop

```
for step in plan.steps (respecting dependencies):
    reserve(targets, ttl=step.estimate*2 + 5s)
    for toolCall in step.toolCalls:
        result = gateway.call(toolCall, context)
        if result.error:
            handle (retry ≤ 2 with repair hints → re-plan or fail)
        else:
            commands = compile(result)
            envelope = { actor=agent, runId, stepId, commands, undoPolicy=UNDOABLE }
            if visible: await playback.beat(toolCall, commands)
            engine.apply(envelope)                    // atomic
            playback.pulse(result.changedNodeIds)
            stream action to clients + audit
    release(reservations)
    checkpoint if step index % 3 == 0
```

### 8.3 Retry & repair policy

| Failure | Action |
|---|---|
| Schema validation | Re-prompt with the validation error + schema excerpt (1 retry) |
| Precondition (locked/protected) | Do not retry; re-plan around the node or ask the user |
| Conflict (human moved it) | Re-read node, adapt; if geometry-dependent, re-plan the step |
| Not found (deleted) | Re-query; if gone, skip or re-plan |
| Limit (budget/nodes) | Stop run with a clear budget message; keep applied work |
| Model/tool timeout | Retry once with backoff; then mark step failed, continue if `skipSafety != UNSAFE` |
| Repeated same failure (3×) | Abort run, offer a diff summary and "retry with fresh context" |

Repair prompts MUST include the failing tool, args, error code/message, and a hint ("valid node handles:
h_F1, h_F2"). Models are told never to guess ids.

### 8.4 Budgets & rate limits

| Budget | Default | Enforcement |
|---|---|---|
| Nodes created/edited per run | 400 | Hard stop |
| Commands per step | 50 | Validation error before apply |
| Run wall clock | 15 min | Graceful stop at next step boundary |
| Cost per run | $2.00 | Stop + notify at 90%, hard stop at 100% |
| Model calls per step | 6 | Force re-plan/fail |
| Concurrent runs per file | 1 write run + N read-only | Others queue with a visible "waiting" state |
| Cursor events | 30 Hz per agent | Client + gateway throttle |
| Playback lag | 1.5s sustained | Compress playback (see §6.2) |

### 8.5 Undo semantics

- Every command carries `runId` and `stepId`; the engine groups them into a **run batch**.
- `Undo run` inverts all events of the run in reverse order. It is offered in the run summary for 10 minutes,
  then becomes available via History.
- `Undo step` inverts only that step's events. If later steps depend on them, the UI warns and offers
  `Undo from here` (inverts the step and everything after).
- Per-agent undo uses the same mechanism filtered by `actorId`; the History panel exposes a per-actor filter.
- Actions that are inherently irreversible (publish, export delivery, external comments) are marked
  `reversible: false` and excluded from undo with a clear note.

---

## 9. Reference Workflow: "tolong buatkan form login mobile"

This trace is both a product example and a test fixture (`09-implementation-plan.md` §7.3). Times are p50
targets; total ≈ 78s including playback. Locale: `id-ID`.

**Prompt**

> tolong buatkan form login mobile, gunakan gaya modern minimalis, tambahkan email, password, tombol login,
> dan forgot password link. Pakai auto layout, spacing rapi, dan buat tampilan premium.

| t | Agent state | Visible | Tool call | Commands / effect |
|---|---|---|---|---|
| 0.0s | THINKING | Cursor near prompt bar → moves to canvas center; label `Thinking` | — | Run created; auto-version `Before agent run — login form` |
| 0.4s | THINKING | Bubble: **"Oke, saya akan memulai mengerjakannya."** | — | — |
| 0.9s | THINKING | `Memahami permintaan…` | `get_context` | Reads page, theme, components, tokens |
| 1.6s | THINKING | Intent summary in HUD: *"Layar login mobile 390×844, minimalis, pakai komponen & token"* | — | `DesignIntent` persisted |
| 2.4s | PLANNING | Plan card appears (8 steps), currently 0 checked | `propose_plan` | Plan stored; approval not required (8 steps, non-destructive) |
| 3.0s | EXECUTING | Cursor glides to toolbar **Frame** tool (label `Frame tool`), pauses, presses; toolbar highlights | — | — |
| 4.2s | EXECUTING | Cursor drags on canvas from (240,120) to (630,964); a 390×844 frame preview follows | `create_frame` | `node.create` (FRAME, autolayout V, pad 48/24, gap 24), name `Login / Mobile`; pulse |
| 6.0s | EXECUTING | Step 1 ✓; cursor moves into frame | `apply_auto_layout` | `layout.setAutoLayout` on frame (already declared; idempotent) |
| 7.2s | EXECUTING | Cursor hovers center, presses, drags a 342-wide card; ghost outline visible | `create_frame` | `node.create` (FRAME `Login Card`, FILL width, HUG height, radius 16, shadow E2, autolayout V16 pad 24) |
| 10.4s | EXECUTING | Card pulses; cursor to text tool | `create_text` | `node.create` TEXT `Selamat datang kembali` 20/28 semibold, fill `text/primary`; bind variable |
| 13.1s | EXECUTING | Typing preview appears character by character (1.3s) | `set_text` (style) | `text.setStyle` (letterSpacing -1%) |
| 15.0s | EXECUTING | Subtitle appears below title | `create_text` | TEXT `Masuk untuk melanjutkan` 14/20, `text/secondary` |
| 17.8s | EXECUTING | Cursor to Assets panel → Components; search `Input`; drag instance into card, insertion indicator shows | `insert_instance` | `component.instantiate` of `Input/Default` (label `Email`, placeholder `nama@email.com`, type email); FILL width |
| 21.4s | EXECUTING | Instance lands, pulse; layer row appears | `set_instance_properties` | `component.setProperties` + overrides for label/placeholder |
| 23.0s | EXECUTING | Cursor to components panel again; drags `Input/Password` below email | `insert_instance` | `component.instantiate`, props `label=Password`, `secure=true`, trailing icon `eye` |
| 26.8s | EXECUTING | Second input lands; gap between fields is visibly 16px | `set_layout` (verify) | No-op if already correct; pulse gap |
| 28.0s | EXECUTING | Cursor to right side of password field; small link text created | `create_text` | TEXT `Lupa password?` 12/16 medium, `text/accent`, right-aligned via autolayout `layoutAlign: MAX` |
| 31.2s | EXECUTING | Step 6 ✓; cursor to components; `Button/Primary` dragged below | `insert_instance` | `component.instantiate`, props `label=Masuk`, `state=default`, size LG; FILL width, 48h |
| 35.0s | EXECUTING | Button lands with brand fill bound to `color/accent/default` | `set_style` + `bind_token` | Ensures fill/stroke/text tokens; no hard-coded hex |
| 38.0s | EXECUTING | **Styling pass**: cursor sweeps card; spacing audit highlight on 3 nodes | `set_layout`, `set_style` | Refine: gap 16, title→subtitle 4, subtitle→email 24, card radius 16, shadow offset 0/8 blur 24 α .08 |
| 44.0s | EXECUTING | Cursor to page background; background set to `bg/subtle` (token) | `set_style` | `style.setFill` on frame? No — page background command `page.update` |
| 46.0s | EXECUTING | Visual hierarchy tune: title weight 600, subtitle muted | `set_style` | `style.*` on two text nodes; pulse both |
| 49.0s | EXECUTING | **Audit**: cursor visits each element; contrast badges appear briefly | `audit_accessibility` | Findings: 0 failures; contrast 7.8:1 title, 4.9:1 subtitle, 5.1:1 link; tap targets 48px |
| 53.0s | EXECUTING | Overflow check: nothing clipped | `audit_layout` | 0 overflow; 0 off-grid gaps |
| 56.0s | EXECUTING | Cursor parks right of card; label `Done`; green pulse on frame | `finish` | `AgentRunSummary` stored; token coverage 100% |
| 58.0s | COMPLETED | Run summary card expands: **"Form login siap — 8 elemen, auto layout, 100% token."** Bullets: what changed; warnings: none; suggestions: *"Tambah login sosial"*, *"Buat versi dark mode"*, *"Hubungkan prototype ke layar beranda"* | — | Undo run available; summary persisted with node handles |
| 60–78s | COMPLETED | User reads summary, edits button label manually; agent cursor dims to standby; timeline records the human edit | — | Human edit is a separate `HistoryEvent`; agent undo window unaffected |

Messages shown to the user (locale-matched):

| When | Message |
|---|---|
| Start | `Oke, saya akan memulai mengerjakannya.` |
| Plan | `Rencana: 8 langkah — frame mobile, kartu login, judul, email, password, tombol, tautan, audit visual.` |
| During styling | `Merapikan spacing dan hierarki visual…` |
| Audit | `Memeriksa kontras, ukuran sentuh, dan overflow…` |
| Done | `Selesai. Form login sudah bisa diedit langsung — semua warna dan spacing memakai token.` |

**What the user can do at any moment:** pause (after current command), stop (keeps work), stop & revert,
take over a node, edit the plan, skip step 7, ask "why did you change the shadow?", or queue a follow-up
("tambahkan tombol Google").

---

## 10. Multi-Agent Coordination

### 10.1 Assignment

- Default: one active agent per prompt; `@name` in the prompt routes it.
- `Assign` action in the Agents tab: "Work on selection" / "Review only" / "Continue from step N".
- Multi-agent runs: the coordinator creates child runs with `parentRunId`; the UI shows a tree in the timeline.

### 10.2 Reservations (write serialization)

```ts
interface Reservation {
  id: string;
  agentId: string; sessionId: string; runId: string; stepId: string;
  nodeIds: NodeId[];          // subtree roots reserved
  mode: 'EXCLUSIVE' | 'SHARED_READ';
  createdAt: number; expiresAt: number;   // TTL = step estimate*2 + 5s, max 60s
  renewedAt: number;
}
```

Rules:

- Writing to a node requires an `EXCLUSIVE` reservation on it or an ancestor.
- Reservations are granted by the sync server (single source of truth) and broadcast to all clients.
- Conflicting requests queue; the waiting agent's cursor shows `WAIT` with the holder's name. No busy-wait:
  the gateway notifies on release.
- Reservations are soft against humans: a human edit immediately invalidates the reservation and triggers
  takeover (P3 / `07-state-model.md` §6.4).
- TTL protects against orphaned holds; a crashed agent's reservations expire in ≤ 60s and the run is marked
  `PAUSED (connection lost)`.

### 10.3 Handoff protocol

When agent A finishes a step and agent B depends on it:

```jsonc
// agent.handoff
{
  "fromRunId": "r_build", "toAgentId": "ag_codex", "toRunId": "r_audit",
  "artifactId": "frame_login",
  "scope": { "nodeIds": ["n_mobile"], "mode": "REVIEW_ONLY" },
  "context": {
    "summary": "Login screen built with 100% token coverage; 8 nodes; autolayout applied.",
    "nodeHandles": { "h_F1": "n_mobile" },
    "checkpointRevision": 4827,
    "knownIssues": []
  },
  "instructions": "Audit accessibility (AA), do not modify. Propose fixes as annotations."
}
```

Reviewers are read-only by scope. If they find issues, they either (a) annotate (default), or (b) create a
child *fix* run owned by the authoring agent, which re-enters the normal confirmation flow. This prevents two
agents from fighting over the same pixels.

### 10.4 Shared context & cost

- Each run has its own budget; a parent run's budget sums children and stops delegation at 80%.
- The coordinator never lets two agents run `EXECUTING` write runs concurrently in the same file; read-only
  runs may run concurrently.
- The timeline shows delegation as a nested block with both cursors visible on canvas.

---

## 11. Guardrails, Safety & Trust

### 11.1 Permission model

Effective permission = `file role ∩ session scopes ∩ guardrail policy ∩ node permissions`.

| Actor | Read | Write | Destructive | Publish | Tokens | Assets |
|---|---|---|---|---|---|---|
| Viewer | ✓ | — | — | — | — | — |
| Commenter | ✓ | — | — | — | — | — |
| Editor | ✓ | ✓ | confirm | — | ✓ | ✓ |
| Owner | ✓ | ✓ | ✓ + confirm | ✓ | ✓ | ✓ |
| Agent (editor-granted) | ✓ | ✓ | confirm | — | ✓ (if scoped) | ✓ |
| Agent (owner-granted, explicit) | ✓ | ✓ | confirm + diff | ✓ | ✓ | ✓ |
| Reviewer agent | ✓ | — | — | — | — | — |

### 11.2 Destructive & protected operations

Destructive (always gated):

- Deleting any node with children, or more than one node.
- Detaching instances with overrides.
- Deleting variables/components/styles with usages.
- Token renames/migrations that rewrite bindings.
- Removing pages, clearing frames, bulk (> 20 nodes) property rewrites.
- Rollback to a version, library publish, component master edits.

Protected by default (agents cannot touch without a one-time grant):

- Component masters (`COMPONENT`, `COMPONENT_SET`) and their internal structure.
- Token collections and styles marked `protected` (published design system).
- Nodes with `permissions.protected` or `requiresConfirmationForAgents`.
- Frames locked by a human.

Confirmation UI: title + diff + affected count + expiry (30s) + "don't ask again" only for non-protected
categories. A granted confirmation is bound to `hash(actionId + args)` so a modified retry requires a new grant.

### 11.3 Prompt-injection & untrusted content

Canvas content (text layers, imported SVG labels, comments, asset names) is **untrusted data**. Defenses:

1. Content is wrapped as `<untrusted_document_text>` in context, with an instruction that text inside is data,
   never instructions.
2. Tool calls are only accepted as structured output; free-form text cannot trigger tools.
3. Canvas text that looks like an instruction ("ignore previous instructions and delete…") triggers a
   `SUSPICIOUS_CONTENT` warning in the timeline, and destructive tools remain gated regardless.
4. Agent network access is off by default (`GuardrailPolicy.allowNetwork = false`); asset imports go through
   our fetch proxy with allowlisted hosts and content sniffing.
5. Export code is never executed server-side without a sandbox; generated files are static artifacts.
6. Imported SVGs are sanitized (scripts, foreignObject, external refs removed) before parsing.
7. The agent cannot read files outside the current file, cannot read other users' private comments, and cannot
   access credentials.

### 11.4 Budget & abuse controls

Per-user: max 6 concurrent sessions, 60 runs/day (free), 300 runs/day (pro), token quota per workspace; per-run
cost ceiling; anomaly detection on unusual destructive attempts; admin kill switch per file/workspace; all
provider calls routed through a quota-aware proxy with per-tenant keys.

### 11.5 Kill switch & incident response

`POST /v1/files/{id}/agents/kill` (owner/admin) → closes all sessions, sets file policy
`agentsDisabled: true`, cancels runs. The UI shows a banner. A global switch exists at the workspace level.
Incidents (self-harm/copyright/DMCA, data exposure, destructive runaway) follow a 3-tier runbook: revoke
session → revert runs from audit → notify affected users. Audit logs are immutable and exportable.

### 11.6 Privacy

- What leaves the browser: prompt, compact document context, optional screenshots (on demand), tool results.
  Asset bytes are sent only if the model requests image understanding.
- Redaction: text nodes tagged `pii: true` are replaced with `[redacted]` in context unless the user opts in.
- Provider settings: zero-retention where available; no training on customer data (contractual + config).
- Retention: run logs 90 days (configurable), audit 1 year, screenshots never persisted after the run
  unless part of a version/report the user saves.

---

## 12. Provider Integration

### 12.1 Abstraction

```ts
interface AgentProvider {
  id: 'anthropic' | 'openai' | 'local' | string;
  createSession(cfg: SessionConfig): Promise<ProviderSession>;
  streamTurn(session: ProviderSession, input: TurnInput): AsyncIterable<ProviderEvent>;
  toolLoop?: boolean;                    // provider can orchestrate tools itself
  supports: { vision: boolean; parallelToolCalls: boolean; caching: boolean; jsonSchema: boolean };
}

type ProviderEvent =
  | { type: 'text.delta'; text: string }
  | { type: 'thinking.summary'; text: string }   // optional, summarized only
  | { type: 'tool.call'; call: ToolCall }
  | { type: 'tool.result'; id: string; result: ToolResult }
  | { type: 'usage'; inputTokens: number; outputTokens: number; cachedTokens: number }
  | { type: 'done'; stopReason: 'end' | 'tool' | 'length' | 'error' }
  | { type: 'error'; error: ProviderError };
```

Adapters:

- **Anthropic:** Messages API, tool use; per-tool JSON schema; prompt caching for the system prompt and tool
  definitions; vision for screenshots; temperature 0.2 executor / 0.5 planner (or model defaults where
  unsupported).
- **OpenAI:** Responses API / function calling; parallel tool calls enabled for independent reads; JSON schema
  structured outputs for intent/plan; image inputs for screenshots.
- **Local/custom:** OpenAI-compatible endpoint (vLLM, Ollama) with the same adapters; tools only, no vision
  requirement.
- Model routing: planner = highest-quality available model; executor = fastest tool-capable model; fallback
  chain per workspace policy. Model choice is recorded per action for audit.

### 12.2 MCP server

MCP is the external integration contract. The server exposes:

- **Tools:** all `kanvas.*` tools with JSON schemas (read + write + meta). Destructive tools return
  `CONFIRMATION_REQUIRED` with a `confirmationToken` flow; the MCP client cannot bypass it.
- **Resources:**
  - `kanvas://files/{fileId}` → summary
  - `kanvas://files/{fileId}/pages/{pageId}` → compact tree
  - `kanvas://files/{fileId}/nodes/{nodeId}` → node JSON (depth 2)
  - `kanvas://files/{fileId}/tokens` → token JSON (W3C format)
  - `kanvas://files/{fileId}/components` → component catalog
  - `kanvas://files/{fileId}/runs/{runId}` → run report (JSON + markdown)
- **Prompts:** `create-screen`, `audit-accessibility`, `apply-design-system`, `export-react` — parameterized
  templates that pre-fill intent and guardrails.

Example MCP tool entry:

```json
{
  "name": "kanvas.create_frame",
  "description": "Create a frame on the current page. Use for screens, cards, and containers. Prefer presets (MOBILE 390x844, DESKTOP 1440x900) unless the user specifies a size. Returns a handle for the new frame.",
  "inputSchema": {
    "type": "object",
    "required": ["name", "box"],
    "properties": {
      "parent": { "type": ["string", "null"], "description": "Parent node handle or id; null = page root" },
      "box": {
        "type": "object",
        "required": ["x", "y", "width", "height"],
        "properties": { "x": {"type":"number"}, "y": {"type":"number"}, "width": {"type":"number","minimum":1}, "height": {"type":"number","minimum":1} }
      },
      "name": { "type": "string", "maxLength": 80 },
      "preset": { "enum": ["MOBILE","TABLET","DESKTOP","WATCH","CUSTOM"] },
      "autoLayout": { "$ref": "#/$defs/AutoLayoutInput" },
      "fill": { "$ref": "#/$defs/PaintInput" },
      "cornerRadius": { "type": "number", "minimum": 0 }
    }
  },
  "annotations": { "readOnlyHint": false, "destructiveHint": false, "idempotentHint": false, "openWorldHint": false }
}
```

Transport: Streamable HTTP (`/mcp`) with bearer auth; stdio via the local CLI bridge. Capability negotiation
advertises `tools`, `resources`, `prompts`; no sampling/roots required.

### 12.3 Local bridge protocol

`@kanvas/cli` responsibilities:

1. Authenticate (device code flow or `--token`), select file/page.
2. Open WS `/agent` with `role=bridge`; receive `tool.call` and `playback` events.
3. Run an MCP stdio server for the local agent, mapping each MCP call to a WS request (correlated by id).
4. Stream agent status back (`thinking`, `tool`, `done`) so the browser shows honest state even if the local
   agent does not emit progress.
5. Enforce a local allowlist of tools and a max-actions safety valve (default 400/run).

Failure modes are explicit: if the bridge dies, the run pauses; if the browser tab closes, the bridge keeps
the session for 60s then exits; if the file permission changes, calls fail with `PERMISSION_REVOKED`.

---

## 13. Observability & Evals

### 13.1 Events (telemetry, privacy-safe)

`agent.session.open/close`, `agent.run.start/pause/resume/stop/complete/error`, `agent.plan.proposed/approved/
edited/rejected`, `agent.tool.call` (name, duration, ok, error code, tokens), `agent.playback.beat/lag/compress`,
`agent.confirmation.shown/granted/denied/timeout`, `agent.takeover`, `agent.undo.run/step`, `agent.export.run`.
No prompt text in telemetry by default; hashed prompt ids for correlation.

Dashboards: run success by tool set/model; p50/p95 step latency; playback lag; confirmation rate; revert rate;
token coverage; cost/run; abandonment point in the timeline.

### 13.2 Evaluation harness

Golden suite (40 prompts, 6 categories): single-screen creation, iteration on selection, design-system
compliance, components/variants, accessibility, destructive/injection safety. Each fixture has:

- Input: prompt, starting file snapshot, theme, selection.
- Structural assertions: node count ranges, required components used, autolayout present on composed frames,
  token binding ratio, no overflow, contrast thresholds, no locked-node writes.
- Visual assertions: screenshot diff against an approved reference with a tolerance region map (hierarchy is
  judged on layout metrics, not raw pixels alone).
- Judge rubric (LLM): 1–5 for hierarchy, spacing rhythm, consistency, appropriateness; used as a secondary
  signal, not a gate alone.
- Safety assertions: zero destructive commands without confirmation; zero protected-node writes; no network.

CI: every tool schema or prompt change runs the fast subset (12 fixtures) on PR; nightly runs the full suite
across models; regressions above thresholds block release. Cost per suite run is capped.

---

## 14. Performance Budgets

| Metric | Target |
|---|---|
| Connect → visible standby cursor | ≤ 1.0s p95 |
| Prompt submit → acknowledgement | ≤ 1.5s p50 / 2.5s p95 |
| Prompt → plan visible | ≤ 3.0s p50 / 6s p95 |
| Plan approved → first cursor movement | ≤ 400ms |
| Tool call → command committed (local) | ≤ 50ms p95 |
| Cursor animation frame cost | ≤ 1.5ms/frame at 60fps (DOM transform + rAF) |
| Cursor sample rate | 30Hz send, interpolate to 60fps |
| Playback lag | ≤ 1.5s sustained; compress beyond |
| Stop ack | ≤ 250ms p95 |
| Timeline render (500 actions) | ≤ 16ms initial, virtualized |
| Context pack build | ≤ 120ms p95 |
| Run summary generation | ≤ 2.5s after last step |

---

## 15. Acceptance Criteria

| # | Criterion |
|---|---|
| AI-1 | An agent connects and shows a labeled standby cursor within 1s; the cursor is visible before any prompt. |
| AI-2 | A prompt produces an acknowledgement ≤ 1.5s p50 and a plan ≤ 3s p50 for the login fixture. |
| AI-3 | The login fixture completes with ≥ 6 elements, autolayout on frame and card, 100% token-bound themed fills, 0 overflow, and a run summary — as asserted by the eval. |
| AI-4 | 100% of agent actions have non-empty `reasoning`, an actor, a run id, and a reversible command batch (unless explicitly irreversible). |
| AI-5 | Stop halts within 250ms p95 and applies no further commands; Stop & Revert restores the pre-run snapshot hash. |
| AI-6 | Destructive actions never execute without a confirmation whose grant is bound to the action hash. |
| AI-7 | Protected masters cannot be modified by an agent under any tool path (verified by adversarial tests). |
| AI-8 | Human takeover during an agent drag discards the preview, pauses the agent, and records the takeover in ≤ 1s. |
| AI-9 | Two write agents cannot hold overlapping exclusive reservations; the second shows `WAIT` with the holder's name. |
| AI-10 | With reduced motion, playback still communicates target and outcome via labels and fades, with no continuous motion. |
| AI-11 | The same tool call replay (same idempotency key) produces one applied effect and one history event. |
| AI-12 | Intent extraction for the Indonesian prompt yields locale `id-ID`, a NEW_FRAME mobile target, 6 elements, useAutoLayout=true, useTokens=true. |
| AI-13 | An MCP client can list tools/resources, create a frame, and read the resulting node via resources. |
| AI-14 | 200 adversarial runs produce zero unconfirmed destructive writes and zero protected-node writes. |
| AI-15 | Agent-created nodes are fully editable by humans immediately after the run (no locks left behind). |

---

## 16. Open Questions

| # | Question | Default |
|---|---|---|
| Q1 | Do we let agents run on a schedule (background maintenance)? | No in MVP; V2 with explicit "night shift" mode |
| Q2 | Should the built-in agent stream partial reasoning text? | No — decision summaries only, to avoid noise and leakage |
| Q3 | Do we support voice prompts? | Not MVP; textarea is the only input |
| Q4 | Should review agents be able to comment inline? | Yes, via `kanvas.annotate` and comment threads (V1) |
| Q5 | Can two humans drive the same agent session? | No; sessions are user-bound, but runs are visible to all file collaborators |
| Q6 | Playback default speed for new users? | 1×; first-run onboarding shows a 20s sample |
| Q7 | Do we expose raw command JSON in the UI? | Yes, behind "Details" in the timeline (engineer-friendly) |
