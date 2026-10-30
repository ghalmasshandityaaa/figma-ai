# 01 — UX Architecture, Layout & Interaction Model

> **Scope:** Information architecture, screen layout, panel/region specifications, component breakdown,
> interaction grammar, keyboard model, agent-facing UI surfaces, motion, accessibility, responsive behavior.
> **Related:** `02-data-model.md` (entities), `03-feature-spec-core.md` (canvas/tool behavior),
> `04-feature-spec-systems.md` (components/tokens/prototype/collab), `05-ai-agent-layer.md` (agent UX &
> motion), `07-state-model.md` (stores).

---

## 1. Design Intent of the App Itself

The editor chrome must recede. It is a **neutral, high-contrast workspace** where the user's artboards and
the agents' cursors are the only colorful things. Two identity colors only:

- **Kanvas accent (iris)** for selection, active tool, primary actions.
- **Agent identity colors** (8-slot palette) for AI collaborators — never used for product chrome.

The chrome must not look "AI-generated": no gradients, no glassmorphism, no emoji in UI, no glowing borders.
Premium here means precision: 1px borders, consistent 4px grid, restrained shadows, crisp typography.

### 1.1 Chrome design tokens

```css
/* Light */
--kv-bg: #FFFFFF;          --kv-panel: #FAFAFB;   --kv-panel-2: #F4F4F6;
--kv-border: #E7E7EA;      --kv-border-strong: #D6D6DB;
--kv-text: #16171B;        --kv-text-muted: #6B6F76; --kv-text-subtle: #9A9DA6;
--kv-accent: #5B5BD6;      --kv-accent-hover: #4F4FD1; --kv-accent-soft: #EEEEFB;
--kv-danger: #D93A3A;      --kv-warning: #B7791F;  --kv-success: #1F8A5B;
--kv-selection: #5B5BD6;   --kv-canvas-bg: #F0F0F2;

/* Dark */
--kv-bg: #0E0E11;          --kv-panel: #15151A;    --kv-panel-2: #1C1C22;
--kv-border: #26262D;      --kv-border-strong: #34343C;
--kv-text: #ECEDEF;        --kv-text-muted: #9A9DA6; --kv-text-subtle: #6B6F76;
--kv-accent: #7B7BEA;      --kv-accent-hover: #8F8FF0; --kv-accent-soft: #20203A;
--kv-canvas-bg: #17171B;
```

| Token group | Values |
|---|---|
| Type | Inter (UI); JetBrains Mono (code, IDs, token names, prompt output). Base 13px / 1.45. Labels 11px/600 uppercase tracking .02em. Panel headings 12px/600. |
| Spacing | 4, 8, 12, 16, 24, 32, 48 |
| Radius | 4 (inputs, chips), 6 (buttons), 10 (cards/menus), 14 (floating bars), 999 (pills) |
| Elevation | E1 `0 1px 2px rgba(0,0,0,.06)` popovers; E2 `0 8px 24px rgba(0,0,0,.12)` floating bars; E3 `0 16px 48px rgba(0,0,0,.18)` modals |
| Focus ring | 2px `--kv-accent` outline, 2px offset, always visible for keyboard focus (`:focus-visible`) |
| Motion | 100ms micro (hover), 160ms controls, 220ms panels, 280ms modals; standard easing `cubic-bezier(.2,0,0,1)`; exit `cubic-bezier(.4,0,1,1)` |

### 1.2 Agent identity palette

Eight distinguishable hues, assigned to agents on connect (first unused; user-overridable). Each has a
text-safe dark variant and a 12% tint for backgrounds/highlights.

| Name | Hex | Tint | Used for |
|---|---|---|---|
| Iris | `#5B5BD6` | `#EEEEFB` | Default first agent |
| Azure | `#0A7EA4` | `#E6F4F9` | 2nd |
| Jade | `#1F8A5B` | `#E6F5EE` | 3rd |
| Amber | `#B7791F` | `#FBF1DF` | 4th |
| Coral | `#C2413B` | `#FBEAE9` | 5th |
| Plum | `#8E3FA8` | `#F5EAF9` | 6th |
| Teal | `#147D7D` | `#E3F5F5` | 7th |
| Slate | `#5B6470` | `#EEF0F3` | 8th |

Human collaborators get the same palette from a separate rotation to avoid collisions; agents always show
a small **spark glyph** in their cursor label so identity is never color-only (accessibility P9).

---

## 2. Information Architecture

```
Kanvas (workspace)
├── File browser (/files)
│   ├── All files, Shared with me, Templates, Trash
│   └── File card actions: open, rename, duplicate, move, archive, delete
└── Editor (/f/:fileId/:pageId?)
    ├── Top bar            file identity, sync, presence, present, share
    ├── Left rail          Pages · Layers · Assets · Components
    ├── Canvas             tools, artboards, cursors, comments, agent playback
    ├── Right panel        Design · Prototype · Agents
    ├── Prompt bar         always visible, pinned bottom-center
    ├── Agent shelf        floating bottom-right presence chips
    └── Overlays           plan card, confirmation dialogs, export, share, version history,
                           command palette, prototype player (full-screen route /present/:fileId)
```

Two modes exist inside the same route:

- **Edit mode** (default): all panels, canvas, prompt bar.
- **Present mode** (full-screen overlay, no chrome): prototype player with device frame, back/restart,
  share, comment pins, and "Return to editor" (Esc). URL is shareable; permissions apply.

**Comments mode** is a canvas tool state, not a separate screen.

---

## 3. Main Editor Layout

### 3.1 Wireframe (1440×900 reference)

```
┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│ ▣  Kanvas   /   Mobile Login    ⌄        ◐ Synced        ⌘K  Search        ▶ Present   Share  ⚙ │ 48
├──────┬──────────────────────────────────────────────────────────────────────────┬───────────────┤
│      │                                                                          │ Design │ Proto│
│  ▤   │                    ┌──────────────────────────────┐                      │───────────────│
│  ◫   │   ┌─────────────┐  │                              │                      │ Frame         │
│  ✦   │   │  Login      │  │        ◇ agent cursor        │                      │ X 24  Y 300   │
│  ◧   │   │  Mobile     │  │        ╭──────────────╮      │                      │ W 342 H 268   │
│      │   │             │  │        │ Adding inputs│      │                      │ ───────────── │
│      │   │  [Title]    │  │        ╰──────────────╯      │                      │ Auto layout   │
│      │   │  [Email]    │  │                              │                      │ ↕ 16  pad 24  │
│      │   │  [Password] │  │                              │                      │ ───────────── │
│      │   │  [ Button ] │  │                              │                      │ Fill  ▣ token │
│      │   │             │  └──────────────────────────────┘                      │ Stroke        │
│      │   └─────────────┘                                                        │ Effects       │
│      │        ┌─────────────┐                                                   │ Typography    │
│      │        │  ◇ Codex    │                                                   │ Export        │
│      │        └─────────────┘                                                   │               │
│      │                                                                          │               │
│      │  ┌──────────────────────────────────────────────────────────────┐        │               │
│      │  │ ✦ Claude Code is working…  Step 4 of 8  ▸ Show plan   ✕ Stop │        │               │
│      │  │ ┌──────────────────────────────────────────────────────────┐ │        │               │
│      │  │ │ Ask Kanvas or @agent…                          ⌘⏎   [→] │ │        │               │
│      │  │ └──────────────────────────────────────────────────────────┘ │        │               │
│      │  └──────────────────────────────────────────────────────────────┘        │               │
│      │                                                       ◉ Maya  ◉ Claude  ◉ Codex │
├──────┴──────────────────────────────────────────────────────────────────────────┴───────────────┤
│  Zoom 82% ⌄    ▦ grid    ◧ snap    100%                          24 fps · 1.2k nodes · 3 agents │
└────────────────────────────────────────────────────────────────────────────────────────────────┘
```

Measurements:

| Region | Size | Resizable | Collapsible |
|---|---|---|---|
| Top bar | 48px | No | No |
| Left rail (tabs) | 44px | No | No |
| Left panel | 240px (200–360) | Yes, drag handle | Yes (rail remains) |
| Canvas | fluid, min 480px | — | — |
| Right panel | 288px (248–420) | Yes | Yes (icon tabs remain) |
| Toolbar (floating) | 44px tall, auto width | No | No |
| Prompt bar | 56px idle / ~96px focused / 148px running | No | Collapses to 44px chip below 1100px |
| Agent shelf | auto, 32px chips | No | Auto-hides when right panel Agents tab open |
| Status bar | 28px | No | Yes |

### 3.2 Region: Top bar

Contents, left → right:

1. **Logo/menu** (Kanvas icon) → app menu: File, Edit, View, Object, Text, Arrange, Agent, Help.
2. **Breadcrumb:** Workspace / File name (inline-editable, click) / current page (dropdown). Dirty state shown
   as `•` before the name until synced.
3. **Sync status** pill: `Synced`, `Syncing…`, `Offline — changes stored locally`, `Conflict — review`.
4. **Search/command** (`⌘K`), placeholder changes with context ("Search layers, components, commands…").
5. **Presence stack:** up to 4 human avatars, then `+N`; hover reveals names/roles; click opens People panel.
6. **Agents chip:** `✦ 2` with status dots (executing = pulsing dot only if reduced motion is off; otherwise static).
7. **Present** (secondary), **Share** (primary).
8. **Dev/utility:** theme toggle (if no forced theme), help `?` (shortcuts, docs, status), avatar menu.

Rules:

- Top bar never contains design controls (those live in the right panel).
- Share is the only filled button in the chrome at rest.
- The bar is a single row at ≥1100px; below that, the search collapses to an icon and presence to `+N`.

### 3.3 Region: Left rail + panels

Rail icons (top → bottom): **Pages**, **Layers**, **Assets**, **Components**, then a divider,
**Design system** (opens the Design System dialog, a full-height sheet), and at the bottom **Help**.

- Rail active state: 2px accent bar on the left edge + icon tint; tooltip with name + shortcut on hover
  (300ms delay).
- Selecting a rail icon toggles the 240px panel; selecting the active icon collapses it.
- `⌥1…⌥5` switches rail panels.

**Pages panel:** flat, ordered list with drag reorder; row = page name, node count, current marker;
footer `+ New page`; right-click menu (rename, duplicate, move, delete, set as home).

**Layers panel:** virtualized tree, 28px rows, 12px indent per level.

```
▾ ⌗ Login / Mobile            390 × 844   ⟳
  ▾ ▣ Login Card              342 × 268   ⬡ instance
      T  Selamat datang…      294 × 28    ƒ token
      ▣ Input / Email         294 × 48    ⬡
      ▣ Input / Password      294 × 48    ⬡
      ▣ Button / Primary      294 × 48    ⬡
  ▸ T  Footer note             342 × 16
```

Row anatomy: expand chevron (12px) · type icon · name (inline edit on double-click) · badges (component,
instance, token-bound, locked, hidden, agent-annotated) · visibility eye (on row hover/focus) · lock.
Selected rows use `--kv-accent-soft`; hovered rows use `--kv-panel-2`. Drag affordances: 2px accent line for
insert-before/after, 1px inset outline + tint for insert-into, 300ms hover to auto-expand.

**Assets panel:** search field, filter chips (All / Images / Icons / Illustrations / Fonts), sort, and a
grid (96px tiles) or list toggle. Upload dropzone covers the panel when dragging files over it. Each tile:
thumbnail, name, dimensions, usage count badge. Context actions: place, replace selection, copy asset ID,
find usages, download, delete.

**Components panel:** search + library selector, local components grouped by folder, drag-to-canvas,
right-click → insert instance, edit master, publish. Variant sets show a stacked-card icon and expand to
variant tiles.

### 3.4 Region: Canvas

- Background: `--kv-canvas-bg` with an optional 8px dot grid at zoom > 200% only (not a decorative grid at
  all zooms — it fights with layout grids).
- Artboards cast a 1px border + E1 shadow; frame name sits above the frame in 12px, muted, hidden below
  40% zoom.
- Rulers: top and left, 20px, appear on `⇧R` or when a guide is dragged; show page coordinates and selection
  extents; origin = page (0,0).
- **Toolbar** floats 12px below the top bar, centered over the canvas: Move (V), Frame (F), Rectangle (R),
  Ellipse (O), Line (L), Text (T), Pen (P), Comment (C), Hand (H), plus a `More` overflow for polygon/star/
  arrow/freeform/slice/measure. Tool buttons 32×32, 4px gap, active = accent fill + white icon.
- **Zoom control** bottom-left: `− 82% +` and a dropdown (25/50/100/200%, Zoom to fit `⇧1`, Zoom to
  selection `⇧2`).
- **Status strip** bottom-right (canvas overlay, 11px, muted): fps meter (dev flag), node count, active
  agents count, sync latency. Hidden by default in production unless "Show performance" is enabled.
- **Comments layer:** pin bubbles 28px, resolved pins 60% opacity; hovering a pin lifts it 2px and reveals
  the first comment line; clicking opens the thread panel anchored to the right of the pin, flipping side
  near the viewport edge.
- **Selection rendering:** 1px accent outline at 100% zoom (1.5px below 50%), 6px square handles (white fill,
  1px accent border), rotation handles offset 16px, radius handles as 6px diamonds, size badge below-right.
  Locked nodes show a dashed outline and a lock glyph on hover. Multi-select shows a bounding box with
  per-node thin outlines.
- **Agent rendering:** agent cursors, drag ghosts, and change pulses live on a dedicated overlay above the
  content but below comments and UI (see `05-ai-agent-layer.md` §6 for visual/motion detail).

### 3.5 Region: Right panel

Tabs: **Design**, **Prototype**, **Agents**. Tab state persists per user per file. The panel scrolls
independently; section headers are sticky (33px) with a disclosure chevron; all sections can be collapsed.

```
┌ Design ──────────────────────────────┐
│ [ Align ][ Distribute ][ Tidy ]      │  ← 32px icon row, disabled when N/A
├──────────────────────────────────────┤
│ Frame                          ⌄     │
│  X 24     Y 300      W 342   H 268   │
│  ◐ 0°     ⟲ ⟳        ⤢ constraints   │
├──────────────────────────────────────┤
│ Auto layout                    ⌄  ⊖  │
│  [↕ Vertical] [⤢ Wrap]              │
│  Gap 16    Padding 24  ⌄             │
│  Align [◱][◲][◳]   [▤ distribute]    │
│  W Fill ⌄   H Hug ⌄    min/max       │
├──────────────────────────────────────┤
│ Fill                           ＋    │
│  ▣ color/bg/surface       100%  ⌄   │
│  └ bound to variable • detach        │
├──────────────────────────────────────┤
│ Stroke · Effects · Typography …      │
├──────────────────────────────────────┤
│ Export                         ＋    │
│  [PNG 2x ▾]  [Preview]  [Export]     │
└──────────────────────────────────────┘
```

Control inventory: see `03-feature-spec-core.md` §10. UX rules that apply panel-wide:

- Numeric fields: label on the left (11px muted), value right-aligned, drag-scrub cursor, arrow keys ±1,
  `Shift` ±10, `Alt` ±0.1; `Enter` commits, `Esc` reverts; invalid input shows a red 1px border and a
  tooltip, never silently clamps.
- Multi-select: shared values shown; differing values show `Mixed` in italic; editing applies to all.
- Bound properties: value shown with a token chip (`color/bg/surface`) and a link icon; the numeric/text
  field is read-only; `⌥click` the chip detaches; a swap icon opens the variable picker.
- Every property row can be "copied as token path" from its context menu (used constantly by engineers).
- Section headers show agent-annotation dots when the last agent run touched that section (with tooltip
  "Changed by Claude Code · 2m ago").

**Prototype tab:** flow list, interaction list for the selected node (trigger + action + transition editor),
"Add interaction" rows, overlay settings, flow start markers toggle, "Present" shortcut.

**Agents tab:** see §5.

### 3.6 Region: Prompt bar (frozen)

"The frozen prompt box" is a **pinned, non-scrolling command surface** anchored to the bottom of the canvas
viewport. It never moves with canvas pan/zoom, never scrolls away, and remains interactive while agents run.

```
IDLE
┌──────────────────────────────────────────────────────────────────────┐
│ ✦  Ask Kanvas to design, edit or audit…        @agent   ⌘⏎    [ → ] │
└──────────────────────────────────────────────────────────────────────┘

FOCUSED (expands, attachments row appears)
┌──────────────────────────────────────────────────────────────────────┐
│ ✦  make the card more premium and reduce spacing                     │
│ ┌──────────────┐ ┌──────────────┐                                    │
│ │ ▣ Login Card │ │ ⌘ selection  │   + attach   /commands   as Claude▾│
│ └──────────────┘ └──────────────┘                                    │
│                                                          Esc  [ → ]   │
└──────────────────────────────────────────────────────────────────────┘

RUNNING (HUD strip above the input; input queues a follow-up)
┌──────────────────────────────────────────────────────────────────────┐
│ ✦ Claude Code  ●  Executing · Step 4/8 · Adding password field       │
│ ████████████░░░░░░░░  00:42                              ⏸  ✕ Stop   │
│ ┌──────────────────────────────────────────────────────────────────┐ │
│ │ Queue a follow-up…                                    ⌘⏎   [+Q] │ │
│ └──────────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────┘
```

Specification:

| Property | Value |
|---|---|
| Anchor | bottom: 16px (24px when status bar visible), horizontally centered over the canvas region, not the window |
| Width | `clamp(480px, 52vw, 760px)`; expands to 820px when focused |
| Height | 56px idle; auto-grow to max 168px as text wraps; running adds 56px HUD |
| Radius / elevation | 14px / E2 |
| Z-index | 40 (below modals at 60, above canvas overlays at 30) |
| Focus shortcut | `⌘/` (also `⌘K` → "Ask agent" command); clicking anywhere on the bar focuses the textarea |
| Send | `⌘Enter` / `Ctrl+Enter`; button disabled when empty or when the selected agent is offline |
| Stop | `Esc` while running (with a 1.2s undo-toast "Stopped — resume?"); `✕` button with confirm only if > 20 actions applied |
| Pause | `⏸` pauses after the current atomic command batch (never mid-command) |
| Queue | Follow-ups allowed while running; shown as chips above the input; max 3 queued |
| Attachments | Selection (auto chip, removable), page (auto), files (drag/paste), screenshot of current viewport |
| Agent target | `as <Agent> ▾` selector, defaults to last-used; `@name` in the prompt overrides |
| History | `↑`/`↓` when input empty; last 50 prompts per file stored locally; "History" popover with search |
| Errors | Inline strip above input: `Agent offline — reconnect`, `No edit permission on this page`, `Run budget exceeded — raise limit in Agent settings` |
| Accessibility | `<textarea>` with label; `aria-describedby` for run status; `aria-keyshortcuts="Meta+Enter"`; live region announces state transitions; focus returns to the bar after a run completes |

Why bottom-center: it keeps the hands near the canvas, matches chat muscle memory, and stays visible while
the right panel is open. It is deliberately **not** a modal: users can keep editing while typing.

**Plan card** (appears directly above the prompt bar, max-height 40vh, own scroll):

```
┌ Plan · 8 steps · est. 45s ─────────────────────────── ⚙ Edit  ⌃ ─┐
│ 1 ✓ Create mobile frame 390×844                                   │
│ 2 ✓ Add centered card with auto layout                            │
│ 3 ✓ Title + subtitle (token: text/primary)                        │
│ 4 ◐ Email + password fields (component: Input/Default)            │
│ 5 ○ Primary button (component: Button/Primary)                    │
│ 6 ○ Forgot password link                                          │
│ 7 ○ Refine spacing + hierarchy                                    │
│ 8 ○ Contrast & overflow check                                     │
│                                                    [Skip] [Approve]│
└───────────────────────────────────────────────────────────────────┘
```

- Steps are checkable/editable (title and detail); drag to reorder; delete = skip.
- `Approve` only blocks execution when `requirePlanApproval` says so (`LARGE`/`ALWAYS`); otherwise the plan
  is shown live and execution proceeds (progressive disclosure: no ceremony for a 3-step task).
- Running plan collapses to a one-line HUD; `Show plan` expands it; the current step is scrolled into view.

---

## 4. Canvas Z-Order & Hit-Testing Contract

| Z | Layer | Tech | Pointer events |
|---|---|---|---|
| 0 | Canvas background | DOM | no |
| 10 | Static content (frames, shapes, raster) | Konva `staticLayer`, `listening: false` | no |
| 20 | Interactive content (text, instances, vectors) | Konva `contentLayer` | yes |
| 30 | Selection, hover, guides, measurement, snap lines | Konva `overlayLayer` (`listening: false`) | no |
| 35 | Agent drag ghost + change pulses | Konva `agentLayer` (`listening: false`) | no |
| 40 | Presence cursors (human + agent) | DOM overlay, transform-synced | cursor labels: yes (hover tooltip), pointer body: no |
| 45 | Comments pins | DOM overlay | yes |
| 50 | Floating canvas UI (toolbar, zoom, agent shelf) | DOM | yes |
| 60 | Prompt bar, plan card | DOM | yes |
| 70 | Menus, popovers, dialogs, toasts | Portal | yes |

Hit-testing rules:

- A node is hit if the pointer is within its rendered bounds (not its bounding box for vectors); tolerance
  4px for edges, 2px for small handles.
- `locked: true` or `visible: false` nodes are skipped by canvas hit-testing.
- Overlapping frames: topmost in z-order wins; `⌘click` cycles deep-select into the deepest child.
- During an agent drag, the dragged node is not hit-testable by humans until the drag completes or is taken
  over (prevents accidental interference); the cursor itself is not a hit target.
- Comment pins win over canvas hits; their hit area is 28px even when the visual is smaller.

---

## 5. Agent UI Surfaces

Agents are visible in four places, each with one job. No surface duplicates another's job.

| Surface | Job | Always visible? |
|---|---|---|
| **Cursor + label** | "someone is here / doing this now" | Yes when connected (standby cursor rests at last position) |
| **Prompt bar + run HUD** | "steer, stop, queue" | Yes |
| **Agent shelf** | "who is connected, what state" | Yes (collapsible), hidden while Agents tab open |
| **Agents tab (right panel)** | "plan, timeline, rationale, history, settings" | No (open on demand, auto-opens on first run of a session) |

### 5.1 Agent shelf

Bottom-right of canvas, above the status bar: one 32px chip per connected agent with avatar/initial, name
(up to 12 chars), and a 6px status dot. Executing chips show a subtle progress ring; paused chips show `⏸`.
Click = focus that agent's run in the Agents tab; right-click = pause/stop/disconnect/set color.

Standby behavior: after 30s idle, chips shrink to avatar-only (name in tooltip). Agents never "hide" entirely
while connected — presence must remain legible (principle P3).

### 5.2 Agent cards (Agents tab header)

```
┌ Claude Code                    ● Executing   ⏸  ⋯ ┐
│ anthropic · claude-sonnet-4-5 · frontend          │
│ scopes: read, write, export      run 4 of 6       │
└───────────────────────────────────────────────────┘
```

Card states map 1:1 to `AgentRuntimeState` (see `05-ai-agent-layer.md` §3) with tokens:

| State | Dot | Label | Card treatment |
|---|---|---|---|
| OFFLINE | hollow gray | Offline | dimmed, Reconnect button |
| CONNECTING | pulsing gray | Connecting… | skeleton line |
| STANDBY | green | Standby | normal |
| THINKING | amber pulse | Thinking… | animated three-dot in label area |
| PLANNING | amber | Planning | plan card preview |
| WAITING_APPROVAL | amber | Needs approval | accent left border + Approve button |
| EXECUTING | iris pulse | Executing · Step 4/8 | progress bar (indeterminate within step) |
| PAUSED | amber | Paused | Resume/Skip buttons |
| BLOCKED | red/amber | Blocked — needs input | reason + action button |
| ERROR | red | Error | message + Retry |
| COMPLETED | green | Completed · 42s | summary link, dismiss |

### 5.3 Agent timeline (Agents tab body)

Chronological, grouped by run; newest run expanded, older runs collapsed to a summary row.

```
Run · "buatkan form login mobile"            14:02 · 48s · $0.11
├─ ✓ 1  Create mobile frame                  kanvas.create_frame      0.8s
│      why: prompt asks for a mobile login screen
├─ ✓ 2  Add card + auto layout                kanvas.apply_auto_layout 1.2s
├─ ✓ 3  Title and subtitle                    kanvas.create_text        2.1s
├─ ◐ 4  Email and password inputs             kanvas.create_component…  3.4s
│      why: reuse Input/Default for consistency with the design system
├─ ○ 5  Primary button
└─ ○ 6  Refine spacing
   [Undo run] [Rerun from step 4] [Export summary]
```

Row anatomy: status icon · step number · step title · tool name (mono, muted) · duration · expand chevron.
Expanded row shows: rationale, affected nodes (chips that hover-highlight on canvas), command list with
before → after values, warnings, and error detail when failed. Rows are keyboard-focusable and expose
`aria-expanded`.

Rows are the **only** place that shows raw tool names and command detail — intentional progressive
disclosure: designers see steps, engineers can inspect commands.

### 5.4 Confirmation & approval dialogs

Destructive or protected actions produce a modal (E3) with:

- Title: verb + object (`Delete 3 frames?`, `Edit protected component "Button/Primary"?`)
- Agent + run context line
- Diff preview (counts + node list, collapsible)
- Primary action in danger style for destructive ops; secondary `Cancel`
- Checkbox `Don't ask again for this agent in this file` (only for non-destructive categories, never for
  protected masters)
- 30s timeout → auto-deny with a timeline entry `Confirmation timed out — skipped`

Blocking a human mid-edit is bad; therefore confirmations target the **agent's** pending action, not the
whole app: the rest of the editor stays interactive, the prompt bar shows `Waiting for your approval`.

### 5.5 Takeover interaction

If a human starts dragging/typing on a node an agent has reserved:

1. Agent immediately stops the in-flight step (no further commands for that subtree).
2. Cursor moves ~24px away from the node, label becomes `Paused — you took over`.
3. A non-modal chip appears in the run HUD: `Resume · Skip step · Undo step`.
4. Timeline records `Human takeover at step 4` with the node and actor.
5. If the reservation is released within 2s and the human makes no further change, the agent may resume
   automatically only when the user clicks Resume (never silently).

---

## 6. Interaction Model

### 6.1 Pointer grammar

| Input | Action |
|---|---|
| Left click | Select node / set tool target |
| Shift+click | Add/remove from selection |
| ⌘/Ctrl+click | Deep select (enter nested container) |
| Double click | Enter container (frame/group) or edit text |
| Drag empty canvas | Box select (with `L` toggled: lasso) |
| Space+drag, middle-drag | Pan |
| Wheel | Pan vertical; `Shift`+wheel pan horizontal; trackpad two-finger pan |
| `⌘`+wheel / pinch | Zoom around pointer |
| Right click | Context menu (selection-aware) |
| Alt+drag | Duplicate and drag |
| Drag from panel/OS | Insert asset, component, file |
| Drag edge/corner | Resize; `Shift` maintains ratio; `Alt` from center |
| Drag rotation zone | Rotate; `Shift` snaps 15°; `Alt` snaps 1° |
| Hover 80ms | Hover outline (suppressed during drag/pan) |

### 6.2 Escape stack (deterministic order)

`Esc` consumes the topmost applicable action, one per press:

1. Close open menu/popover/tooltip.
2. Cancel active drag/transform (restore original geometry).
3. Exit text editing (commit).
4. Clear active tool → Move tool.
5. Clear selection.
6. If agent run is executing: **Stop run** (with resume toast). This comes after tool/selection reset so
   editors do not accidentally kill an agent while trying to deselect.
7. Exit focus mode / close side panels if they were keyboard-opened.
8. Nothing else — `Esc` never closes the file or navigates away.

### 6.3 Keyboard model (canonical subset)

| Keys | Action | Notes |
|---|---|---|
| `V F R O L T P C H` | Tools | `Shift+L` arrow, `Shift+R` slice, `Shift+T` measure |
| `⌘Z` / `⌘⇧Z` | Undo / redo | Per-actor history when agent is focused in timeline |
| `⌘G` / `⌘⇧G` | Group / ungroup | |
| `⌘D` | Duplicate | 8px offset |
| `⌘C ⌘V ⌘X` | Copy/paste/cut | Internal clipboard keeps node subtree + overrides |
| `⌘⌥C` / `⌘⌥V` | Copy/paste properties | Includes token bindings |
| `⌘⌥K` | Create component | Or from selection |
| `⌘/` | Focus prompt bar | |
| `⌘K` | Command palette | |
| `⌘⏎` | Send prompt | Only when prompt focused |
| `⌘⇧E` | Export dialog | |
| `⇧1` / `⇧2` | Zoom to fit / selection | |
| `⇧R` | Toggle rulers | |
| `⇧G` | Toggle layout grid | |
| `⌘;` | Toggle guides | |
| `⌥1…⌥5` | Switch left panel | |
| `⌥A` | Agents tab | |
| `⌘⌥P` | Present | |
| `⌘\` | Hide all panels | Focus mode |
| `Tab` / `⇧Tab` | Select next/prev sibling | Inside canvas focus |
| `↑↓←→` | Nudge 1px | `Shift` = 10px |
| `⌘[` / `⌘]` | Send backward / bring forward | `⌘⌥[`/`]` to back/front |
| `⌘R` | Rename selected | |
| `⌘⇧H` | Show/hide selected | |
| `⌘⇧L` | Lock/unlock selected | |
| `⌘F` | Find layers | |
| `?` | Shortcuts sheet | |

Full, per-area shortcut tables: `03-feature-spec-core.md` §12.

### 6.4 Focus & keyboard navigation

- The canvas is a single focusable region (`role="application"`, `aria-label="Design canvas"`); when focused,
  it shows a 2px inset focus ring and exposes the selection tree to assistive tech.
- `Tab` inside the canvas moves focus into the **layer tree mirror** (an accessible, visually hidden tree that
  stays in sync with selection), not through every node in the canvas. This is the only sane model for
  thousands of nodes: screen readers navigate the tree; the canvas is for spatial manipulation.
- `Tab` from the canvas moves to the left panel, then the right panel, then the prompt bar, then back.
  Panel-internal focus uses roving tabindex and arrow keys.
- All drag interactions have a keyboard equivalent: `Enter` on a selected node opens a "Move & resize" popover
  (x/y/w/h inputs); arrow keys + `Alt` modifiers move; `⌥⌘` arrows resize.
- Focus is never trapped except in modals; modals restore focus to their trigger on close.

### 6.5 Context menus

Right-click targets (top-level items only; full lists in feature specs):

- **Empty canvas:** Paste here · Paste over selection · Create frame from selection · Select all · Toggle rulers ·
  Toggle grid · Agent actions (Ask agent to…, Run accessibility audit here).
- **Frame:** Rename · Frame selection · Apply auto layout · Add layout grid · Copy as PNG/SVG · Add interaction ·
  Set as prototype start · Protect frame · Ask agent about this frame.
- **Node (general):** Copy · Copy as PNG/SVG · Duplicate · Delete · Group · Ungroup · Bring forward/back ·
  Lock/unlock · Hide · Add to component · Bind token (fill/stroke/…) · Ask agent / Explain change.
- **Instance:** Go to master · Detach · Reset overrides · Swap component · Publish changes · Show component props.
- **Text:** Edit text · Copy as text · Convert to component · Create text style · Find and replace.
- **Agent timeline row:** Show on canvas · Copy command JSON · Retry step · Undo step · Ask why.

Menus show shortcuts right-aligned; disabled items include a reason tooltip ("Locked by Maya").

### 6.6 Command palette (`⌘K`)

Sections: **Ask agent** (prompt draft), **Run command** (editor commands), **Go to** (page/frame/layer),
**Insert** (component/asset), **Agent** (pause all, undo last run, open timeline), **Export**.
Query syntax supports `>` for commands, `@` for components, `#` for pages, `?` for help. Ranking = recency ×
fuzzy score; no more than 8 rows per section. Empty state shows last 5 actions.

### 6.7 Drag & drop from outside the browser

Files dropped on the canvas: images/SVG place at drop point (SVG imported as vector if parseable), `.kanvas`
opens a new file tab (with unsaved-changes guard), text/URLs create a text layer or image (for direct image
URLs). Drop targets highlight: canvas (insert), left Assets panel (add to library), layer row (reparent).

---

## 7. Screen States & Feedback Patterns

### 7.1 Loading

- File open: canvas skeleton with frame outlines shimmer (max 600ms), panels populated from cached local state
  instantly; a 2px top progress bar for asset/page loads.
- Fonts: text renders in fallback immediately with a subtle `Font loading` badge in the inspect panel; swap
  with no layout jump where possible (metrics-compatible fallbacks, `font-display: swap`).
- Agent run: HUD shows `Thinking…` with elapsed time; no fake progress percentages; step-level progress only.

### 7.2 Empty states

| Surface | Empty copy | Primary action |
|---|---|---|
| New file canvas | "Start with a frame, or ask an agent to sketch one." | `Create frame` / prompt suggestions |
| Layers | "Nothing here yet. Draw something or paste a design." | `Create frame` |
| Assets | "Drop images or icons here to reuse them across the file." | `Upload` |
| Components | "Turn any frame or group into a reusable component." | `Create component` |
| Agents | "Connect Claude Code, Codex, or use the built-in agent." | `Connect agent` |
| Comments | "No comments yet. Press C and click anywhere." | — |
| Agent timeline (new) | "Runs will appear here with every step, change, and reason." | `Try an example prompt` |

### 7.3 Toasts

Bottom-left, 24px from the edge, stacked max 3, auto-dismiss 4s (8s for errors, never for confirmations).
Types: info, success, warning, error, agent. Agent toasts include agent identity color and a "View" action
linking to the timeline. Undoable actions show `Undo` inline (10s). Errors are dismissible and never cover the
prompt bar or the agent shelf.

### 7.4 Offline & reconnect

- Top bar sync pill becomes `Offline` with tooltip; editing continues locally (Yjs local persistence).
- Banner (32px, top of canvas): `You're offline. Changes are saved on this device and will sync when you reconnect.`
- On reconnect: `Reconnected — syncing 12 changes…`, then `Synced`.
- If the agent session drops mid-run: run state becomes `PAUSED (connection lost)`, cursor disappears after
  3s with a fading label `Claude Code disconnected`, HUD shows `Reconnect agent` (no auto-resume of edits).

### 7.5 Permission-denied UI

Never silently disable. Show a lock icon and a tooltip with the reason and the actor who locked it. Attempted
edits produce a toast: `Locked by Maya — ask for edit access`. Agents attempting protected edits produce a
`BLOCKED` timeline row with the guardrail that fired.

---

## 8. Responsive Behavior

| Viewport | Layout |
|---|---|
| ≥ 1440px | Reference layout; all panels open |
| 1280–1439 | Right panel 272px; prompt 560px |
| 1024–1279 | Left panel overlays the canvas when opened; right panel default collapsed to tab rail; prompt bar 480px |
| 768–1023 | **Viewer mode**: canvas read-only, panels become sheets, comments + prototype + agent monitor available; banner "Editing requires a larger screen" |
| < 768px | Viewer mode with bottom-sheet panels; comment composer full-width; no canvas editing |

Rationale: precise multi-select and property editing with a mouse/trackpad is the job; a broken mobile editor
is worse than an honest viewer. Tablets with a keyboard + trackpad are treated as 1024+.

Pointer/touch:

- Touch: one finger pans (not draws), two-finger pinch zooms, long-press opens context menu, toolbar tool
  required for drawing. This prevents accidental edits and matches viewer-first mobile.
- Stylus: pressure captured for freeform (if `PointerEvent.pressure` available), palm rejection heuristics
  (ignore `pointerType === 'touch'` while pen is active within 500ms).

---

## 9. Motion & Feedback

### 9.1 UI motion

| Element | Motion |
|---|---|
| Panel open/close | width 220ms standard ease; content cross-fades 120ms (no horizontal text reflow animation) |
| Popover/menu | 120ms opacity + 4px translateY; origin-aware |
| Modal | 220ms opacity + scale .98→1 (E3 shadow fades in) |
| Tool switch | none (instant), active state only |
| Toast | 180ms slide-in from left + fade |
| Selection change | instant (no animation) — precision work must not feel laggy |
| Zoom | direct manipulation; programmatic zoom-to-fit animates 240ms with ease-out-expo, interruptible by user input |
| Frame navigation (present) | per prototype transition spec |

Rule: **never animate geometry the user is editing.** Only chrome animates. Agent motion is the exception and
is specified separately (`05-ai-agent-layer.md` §6).

### 9.2 Reduced motion

Honors `prefers-reduced-motion` and an in-app setting (`Settings → Accessibility → Reduce motion`). When on:

- Agent playback: cursors jump between labeled checkpoints with a 120ms fade, 200ms hold, and always show the
  tooltip/step label; drag ghost becomes a 2px outline move with no trail.
- Panel/modal animations become opacity-only 80ms.
- Zoom animations become instant.
- Progress pulses become static text (`Executing…`).
- Canvas change pulses remain (they are information, not decoration) but use opacity fade instead of scale.

---

## 10. Component Breakdown (React)

All components are client components. Data access is via stores/hooks (`07-state-model.md` §2), never direct
fetch inside leaf components. Panels use `ResizeObserver` + CSS variables for sizes. Lists use virtualization
(`@tanstack/react-virtual`). All interactive elements have visible focus and accessible names.

| Component | Responsibility | Key deps |
|---|---|---|
| `AppShell` | Route-level layout, panel sizing, keyboard router mount | `useUIStore` |
| `TopBar` | File identity, sync, presence, share/present | `useFileStore`, `usePresence` |
| `FileBreadcrumb` | Inline rename, page switcher | `useFileStore` |
| `SyncStatus` | Connection/persistence state | `useSyncStore` |
| `PresenceStack` | Human avatars, overflow, follow | `usePresence` |
| `AgentStatusChip` | Connected agent count + state | `useAgentStore` |
| `LeftRail` / `LeftPanel` | Tab switch, resize, collapse | `useUIStore` |
| `PagesPanel` | Page CRUD/reorder | `useDocumentStore` |
| `LayersPanel` | Virtualized tree, search, DnD | `useDocumentStore`, `useSelectionStore` |
| `LayerRow` | Row rendering, badges, inline rename | memoized |
| `AssetsPanel` / `AssetTile` | Library, upload, DnD | `useAssetStore` |
| `ComponentsPanel` / `ComponentTile` | Component browser, DnD | `useLibraryStore` |
| `CanvasViewport` | Viewport events, zoom/pan, coordinate transforms | `useViewportStore` |
| `KonvaStage` | Scene graph mount, layer sync | document projection |
| `StaticContentLayer` / `ContentLayer` | Render nodes by type | `nodeRenderers` |
| `SelectionOverlay` | Outlines, handles, size badge | `useSelectionStore` |
| `SnapOverlay` | Alignment guides, measurements | `useSnapStore` |
| `RulerLayer` | Rulers, guides, origin | `useViewportStore` |
| `Toolbar` / `ToolButton` | Tool selection, overflow | `useToolStore` |
| `ZoomControl` | Zoom presets, fit | `useViewportStore` |
| `CanvasStatusStrip` | fps/nodes/sync (dev flag) | telemetry |
| `HumanCursorLayer` / `RemoteCursor` | Presence cursors with interpolation | `usePresence` |
| `AgentCursorLayer` / `AgentCursor` | Agent cursors, labels, tooltips, drag ghost, pulses | `useAgentPlayback` |
| `CommentsLayer` / `CommentPin` / `CommentThreadPopover` | Comment pins and threads | `useCommentStore` |
| `RightPanel` | Tabs and sections | `useUIStore` |
| `DesignTab` → `AlignmentSection`, `TransformSection`, `LayoutSection`, `FillSection`, `StrokeSection`, `EffectsSection`, `TypographySection`, `ConstraintsSection`, `ComponentSection`, `TokenBindingRow`, `ExportSection` | Property editing | `useDocumentStore`, `useTokenStore` |
| `PrototypeTab` → `FlowList`, `InteractionEditor`, `TransitionEditor`, `OverlayEditor` | Prototype authoring | `usePrototypeStore` |
| `AgentsTab` → `AgentCard`, `AgentTimeline`, `ActionRow`, `PlanSummary`, `RunSummary`, `AgentSettings` | Agent observability/control | `useAgentStore` |
| `PromptBar` → `PromptInput`, `AttachmentChips`, `AgentTargetSelect`, `SlashMenu`, `RunHUD`, `QueueChips` | Prompt + run control | `usePromptStore`, `useAgentStore` |
| `PlanCard` / `PlanStepRow` | Plan review/approval | `useAgentStore` |
| `ConfirmationDialog` | Guardrail confirmations | `useAgentStore` |
| `AgentShelf` | Compact agent presence | `useAgentStore` |
| `CommandPalette` | Global command search | `useCommandRegistry` |
| `ExportDialog` / `ExportJobProgress` | Export config + progress | `useExportStore` |
| `ShareDialog` / `PeoplePanel` | Permissions, links, agents access | `useFileStore` |
| `VersionHistoryPanel` | Timeline, restore, diff | `useHistoryStore` |
| `PrototypePlayer` | Full-screen player runtime | `usePrototypeRuntime` |
| `ToastCenter` | Notifications | `useToastStore` |
| `ShortcutsSheet`, `HelpMenu`, `SettingsDialog` | Support surfaces | — |
| `A11yTreeMirror` | Hidden accessible layer tree for screen readers | `useDocumentStore` |

Rendering rules:

- `LayerRow`, `Cursor`, `Handles`, `ActionRow` are memoized and subscribe to narrow selectors.
- Cursor overlays are rendered in a single React root with imperative transform updates via refs (no re-render
  per frame).
- Konva nodes are created declaratively by `react-konva` but updated imperatively for drag/transform
  (`node.x()`, `node.rotation()`, `layer.batchDraw()`), then committed as one command on drag end.

---

## 11. Onboarding & Learnability

1. **First-run file** contains a 2-frame sample (a login screen and its dark theme) plus a pinned note frame
   "Try: ask the agent to make this premium".
2. **Checklist card** (bottom-left, dismissible): Create a frame · Add auto layout · Connect an agent · Run a
   prompt · Export. Progress persisted.
3. **Agent connect dialog** supports: built-in agent (one click, no key needed on hosted plan), Claude Code
   (copy `npx kanvas connect` command / open local bridge), Codex (API key or CLI bridge), MCP URL.
4. **First run experience:** on first agent run, the timeline auto-opens with a 3-line explainer (plan →
   cursor → undo). Dismissed forever after.
5. **Shortcut hints** appear in tool tooltips; `?` opens the full sheet. No modal tours.

---

## 12. Accessibility Requirements (UX-level)

| Requirement | Implementation |
|---|---|
| WCAG 2.2 AA | Editor chrome and prototype player audited; tokens meet 4.5:1 text / 3:1 UI contrast |
| Keyboard parity | Every action in the app is reachable via keyboard; documented shortcut map; skip-to-canvas link |
| Canvas semantics | Hidden accessible tree mirrors layers (`role="tree"`, `aria-selected`, `aria-expanded`), actions via context menu key |
| Agent accessibility | Status changes announced via polite live region at **step boundaries only** (never per cursor move); run summary is a focusable region; "Explain this change" available on every action row |
| Screen reader labels | Icon buttons have `aria-label`; numeric fields have units in labels ("Width, pixels"); toasts use `role="status"`; errors `role="alert"` |
| Focus visibility | 2px accent ring with 2px offset on all focusable elements; never `outline: none` without replacement |
| Hit targets | ≥ 24×24 CSS px (AA 2.2); toolbar buttons 32×32; layer row controls 24×24 with 4px spacing |
| Text scaling | Chrome usable at 200% zoom; panels reflow, no clipped labels; canvas text unaffected (it is content) |
| Color independence | Agent identity shown by spark glyph + name, not color alone; status shown by dot **and** label |
| Reduced motion | §9.2; also disables cursor trails and parallax; prototype transitions replaced with dissolve ≤ 120ms |
| High contrast | `prefers-contrast: more` raises border contrast to 3:1, removes subtle tints, thickens focus rings to 3px |
| Captions/alt | Image nodes have `alt` metadata; prototype player exposes it; export carries it into code |
| Timing | No time-limited interactions except confirmation timeout (30s) which has an explicit "keep waiting" option |

Agent-specific a11y: the agent can run an **Accessibility audit** producing annotations (contrast, tap target,
missing alt, heading order, focus order) and a report; it may propose fixes but must apply them only after
confirmation if they alter > 10 nodes (guardrail).

---

## 13. UX Acceptance Criteria

| # | Criterion |
|---|---|
| UX-1 | A Figma user can create a frame, add text, apply auto layout, create a component, and export PNG without documentation (usability test, 5/5 users). |
| UX-2 | The prompt bar is reachable in one shortcut (`⌘/`), always visible at ≥1024px, and never covers the selection's properties panel. |
| UX-3 | An agent cursor is visible within 1s of connection in standby, labeled with the agent name, at the last active position. |
| UX-4 | Submitting a prompt shows acknowledgement ≤ 1.5s and a plan (or first step) ≤ 3s p50. |
| UX-5 | Stop takes effect ≤ 250ms p95; no command applies after acknowledgement. |
| UX-6 | Every agent action row is expandable to rationale + tool + changed nodes; 100% of actions have non-empty reasoning. |
| UX-7 | A human can take over an agent-edited node in one drag; the agent pauses and the timeline records the takeover. |
| UX-8 | All panels and dialogs are keyboard navigable; axe reports zero critical violations on editor chrome and player. |
| UX-9 | With reduced motion on, agent playback still communicates target and result (label + pulse) without continuous motion. |
| UX-10 | At 1024px width, the editor is fully usable with the right panel collapsed; at 768px, editing is disabled with an explicit explanation. |
| UX-11 | Undo of an agent run restores the file to the pre-run revision (verified by snapshot hash) and is reflected in all collaborators within 2s. |
| UX-12 | No agent surface uses color alone to convey identity or state. |

---

## 14. Open Questions

| # | Question | Owner | Default if unresolved |
|---|---|---|---|
| Q1 | Should the plan card block execution by default for runs > 12 steps? | Product | Yes (`requirePlanApproval: LARGE`) |
| Q2 | Do we allow multiple prompt bars (per agent)? | Design | No — one bar, agent selector + `@mention` |
| Q3 | Should the agent timeline be shareable as a read-only link? | Product | V1 yes, stored as a run report |
| Q4 | Persistent agent cursor position across sessions? | Design | Remember last position per file, restore on connect |
| Q5 | Left panel default: Layers or Pages? | Design | Layers |
| Q6 | Show agent token/cost usage in the timeline by default? | Product | Collapsed under "Details"; visible to workspace admins |
