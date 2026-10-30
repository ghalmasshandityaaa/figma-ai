# 00 — Product Vision, Principles & Scope

> **Product:** **Kanvas** (working name — Indonesian for *canvas*) — an AI-native design workspace.
> **One line:** Figma-grade design editing where AI agents are visible, interruptible collaborators
> working inside the same canvas as humans.
> **Document owner:** Product. **Status:** Approved for specification. **Supersedes:** n/a.

---

## 1. The Thesis

Design tools have spent a decade making **humans faster at manipulating pixels**. AI has spent the last
three years making **machines good at generating pixels**. Neither has solved the actual problem:

> The work of design is not generating a screen. It is **deciding, arranging, revising, and handing off**
> — and that work is collaborative, iterative, and full of context that lives in the file.

Two failure modes dominate today:

1. **Prompt-only generators** (v0-style) produce a screen you cannot really edit. The output is a
   screenshot with a "download code" button. The moment a designer wants to move a button, the tool ends.
2. **Chat-only agents bolted onto editors** make changes invisibly. The user types, waits, and receives a
   diff they did not watch. Trust collapses the first time the agent deletes a frame.

Kanvas takes the third path: **agents operate the editor, visibly, through the same commands humans use.**

```
Human intent ──▶ Prompt / direct manipulation
                        │
                        ▼
              ┌───────────────────┐
              │   Design Engine    │  commands · history · layout · tokens
              └─────────┬─────────┘
                        │ structured, attributable mutations
        ┌───────────────┼────────────────┐
        ▼               ▼                ▼
   Human cursor    Agent cursor     Realtime sync
   (canvas)        (animated,       (CRDT, presence,
                    explainable)     comments)
```

The design file is the shared substrate. The agent is not a separate product that spits out a result; it is
a **teammate with a cursor, a plan, a permission set, and an audit trail**.

### 1.1 What "AI-native" means here (and what it does not)

| AI-native **is** | AI-native **is not** |
|---|---|
| Agents mutate the same document graph humans edit | A separate generator that produces images |
| Every agent action is a typed, undoable command | Opaque "magic" edits applied on a server |
| Plans, rationale, and progress are visible | A spinner followed by a finished screen |
| Agents can be stopped, paused, corrected mid-task | Fire-and-forget prompts |
| Agents respect locks, permissions, and design systems | Agents that overwrite anything they can reach |
| Agents are addressable collaborators (`@claude refine this`) | A single chatbot panel detached from the work |

---

## 2. Vision Statement

> **In 3 years, a designer opens Kanvas, sketches the skeleton themselves, and hands the tedious 60% —
> states, spacing, responsive variants, tokens, accessibility passes, code export — to agents they can
> watch, steer, and trust. The file remains a real design file the whole time.**

The long-term wedge is not "AI makes designs". It is **"AI does design work in a file you own, with a
review process you control."** That is what makes it adoptable by professional teams rather than a toy.

### 2.1 Strategic pillars

1. **Editor credibility first.** If manual editing is not as good as Figma for the core 20% of workflows,
   no agent feature matters. The canvas, text, auto layout, components, and tokens must be genuinely good.
2. **Visible agency.** The cursor, plan, and action log are product features with the same care as the
   canvas — not debug UI.
3. **Composable intelligence.** Kanvas does not bet on one model vendor. Claude Code, Codex, and any
   MCP-capable agent connect through one protocol; a built-in orchestrator covers users without a CLI.
4. **System-level output.** The differentiator is not prettier screens; it is designs that are
   token-bound, componentized, accessible, and exportable to production code.
5. **Trust by construction.** Attribution, confirmation gates, per-agent undo, and protected masters are
   architectural, not policy.

---

## 3. Target Users & Jobs

### 3.1 Primary personas

| Persona | Context | Core job | What must be true for them |
|---|---|---|---|
| **Maya — Product Designer** (primary) | 5-person startup, owns end-to-end flows, knows Figma deeply | "Get from idea to polished, developer-ready screens without doing every state by hand" | Canvas parity, components/tokens, agent that follows her system and never surprises her |
| **Bima — Frontend Engineer** | Receives designs, builds components, hates redline drift | "Get accurate structure, tokens, and code — not a screenshot" | Inspect panel, token export, code export that compiles, comments |
| **Sari — Founder / PM** (secondary) | Non-designer, needs to validate an idea fast | "Turn a sentence into a credible clickable prototype" | Prompt → editable design in < 2 min, templates, safe defaults |
| **Devi — Design Systems Lead** (secondary) | Agency/scale-up, maintains libraries | "Keep 40 files consistent, publish changes safely" | Tokens, library versioning, protected masters, migration tooling |
| **The Agent** (actor, not persona) | Claude Code / Codex / built-in | "Execute design intent with minimal ambiguity, report honestly" | Compact context API, typed tools, deterministic layout, clear failure modes |

### 3.2 Jobs to be done

- *When I have a clear layout in my head,* I want to build it fast by hand without fighting the tool.
- *When I need 12 variations of a card,* I want an agent to produce them inside my system, and I want to
  see it working so I can intervene early.
- *When an agent changes my file,* I want to know exactly what changed, why, and be able to undo just that.
- *When I hand off,* I want tokens, components, and code that match the design without a translation layer.
- *When two humans and two agents edit one file,* nothing should silently clobber anything.

### 3.3 Anti-personas (explicitly not optimized for)

- Users who want a finished marketing video or photorealistic imagery (non-goal).
- Enterprise procurement teams needing SSO/SCIM/on-prem on day one (roadmap, not MVP).
- Users who want a chat-only experience with no canvas (different product category).
- Print/prepress production (CMYK, bleed, imposition) — out of scope.

---

## 4. Product Principles (normative)

Each principle has a **mechanism** and an **enforcement point**. If a feature cannot satisfy these, it does
not ship.

### P1. Human familiarity, not novelty

The editor must feel learnable to a Figma user in under 10 minutes: same spatial layout, same core
shortcuts (V, F, R, O, T, P, ⌘G, ⌘Z), same selection grammar, same properties panel logic.

- **Mechanism:** Figma-compatible shortcut map, familiar panel arrangement, familiar terminology.
- **Enforcement:** usability gate — a Figma user completes "create frame → add text → autolayout → make
  component → export PNG" unaided in the beta test. We do not invent new interaction metaphors for core work.

### P2. Every change is attributable and reversible

No anonymous mutations. Commands carry `actorId`, `actorType`, `runId`, and produce a `HistoryEvent` with
inverse commands.

- **Enforcement:** `CommandEnvelope.actorId` is required by the type system and rejected by the gateway if
  absent; `HistoryEvent.inverse` is validated by replaying it in CI.
- **Consequence:** "Undo agent's last run" is a first-class button, not a reconstruction.

### P3. Visible work beats fast work

An agent that appears to work builds trust; a fast agent that teleports edits does not. Default is
**hybrid execution**: commands are computed instantly, but presentation is choreographed.

- **Mechanism:** `CursorPlaybackScript` per action; smooth interpolation; labeled cursor; change pulse.
- **Enforcement:** any new agent tool must declare a playback template or an explicit
  `presentation: 'SILENT'` with a reason shown in the timeline.
- **Escape hatch:** users can set agent speed to `Instant` (accessibility + power users). The action log
  remains complete.

### P4. Control is always one gesture away

Stop, pause, take over, edit the plan, skip a step, revert a run.

- **Mechanism:** persistent run HUD in the prompt bar; `Esc` stops; canvas grab during an agent edit triggers
  a takeover handshake; plan approval gates for large/destructive runs.
- **Enforcement:** stop acknowledgement < 250ms server-side; no command is applied after `run.stop` is
  acknowledged. Agents never "finish the current batch" beyond a 1-command atomicity boundary.

### P5. Explain, don't narrate

Explainability means *causal specificity* ("moved the button into the card's auto layout so it spans 294px"),
not stream-of-consciousness. Every action carries a one-sentence rationale; plans carry step intent.

- **Mechanism:** `AgentAction.reasoning` is mandatory; UI shows it inline; empty reasoning fails schema
  validation for `TOOL_CALL`/`COMMAND_BATCH`.
- **Anti-pattern:** verbose chain-of-thought dumps. We surface decisions, not tokens.

### P6. Safe by default, permissive by consent

Destructive operations, protected masters, token rewrites, and library publishes require explicit grants.

- **Mechanism:** `GuardrailPolicy` per file + per agent scopes + per-action confirmation.
- **Rule:** agents may always *read*; may *write* within scopes; must *ask* for destructive, publish, and
  protected-node edits; may never bypass a human lock.

### P7. The design system is the contract

Agents are evaluated on using tokens and components, not on raw pixel output. A visually correct screen
with hard-coded colors is a failure.

- **Mechanism:** token-first tool defaults; lint after each run (`hardcoded-color`, `off-grid-spacing`,
  `contrast`, `missing-component`); summary reports violations.
- **Enforcement:** agent eval fixtures assert token binding ratio ≥ 0.9 for themed surfaces.

### P8. Performance is a design feature

Canvas interactions target 60fps; agent playback must never degrade human editing. Presence and cursor
traffic are throttled and interpolated; heavy work runs off the main thread.

- **Budgets:** see §8 and `06-architecture.md` §9.

### P9. Accessible by default, not as an afterthought

The editor chrome, the prototype player, and generated output meet WCAG 2.2 AA. Agents can run an
accessibility pass; the editor exposes semantics (heading levels, alt text, roles) as first-class node
metadata.

- **Mechanism:** keyboard-only path for every action; reduced-motion mode changes agent playback; screen
  reader announcements for agent state; contrast checker in the color picker.

### P10. Open over locked-in

Files export to open formats; agents connect through an open protocol; code exports are idiomatic and
framework-native. No proprietary runtime required to view an exported design.

---

## 5. Use Cases (canonical journeys)

### 5.1 Manual design (baseline)

Maya creates a 390×844 frame, adds a title, inputs, and a button, applies auto layout, converts the button
to a component, binds colors to tokens, prototypes the success state, exports PNG for a stakeholder.

**Success:** every step is discoverable without docs; median time to first frame < 60s for a Figma user.

### 5.2 AI-assisted design (hero journey)

Sari types: *"tolong buatkan form login mobile, gunakan gaya modern minimalis, tambahkan email, password,
tombol login, dan forgot password link. Pakai auto layout, spacing rapi, dan buat tampilan premium."*

Flow:

1. Agent acknowledges (`Oke, saya akan memulai mengerjakannya.`) near its cursor.
2. Plan appears in the right rail in < 3s; for this size, execution starts without a blocking approval.
3. Cursor moves to the frame tool, drags a mobile frame, then works inside it: title, inputs, button, link.
4. Auto layout is applied and spacing refined; tokens are bound; contrast is checked.
5. A run summary lists what changed, why, warnings, and next suggestions.
6. Sari edits the button label by hand immediately after; nothing is locked.

**Success:** prompt → editable, token-bound, auto-laid-out login screen in < 90s, ≥ 6 elements, zero
hard-coded colors, all changes undoable as one run.

### 5.3 AI iteration

Maya selects a pricing card and types: *"make it more premium and reduce spacing."* Scope = selection.
The agent proposes a 3-step plan (visual audit → spacing/typography changes → contrast check), executes
visibly, and shows a before/after diff summary. Maya rejects step 2, edits spacing manually, reruns step 3.

**Success:** selection-scoped runs never touch outside the selection unless the plan says so and the user
approves; diff is reviewable.

### 5.4 Multi-agent

Two agents are connected: *Claude Code (layout)* and *Codex (a11y)*. The user assigns: "Claude, build the
settings screen; Codex, audit it when done." Claude creates; on completion, the orchestrator starts Codex
with the produced node handles as context. Both cursors are visible; Codex's cursor hovers each audited
element and drops annotation pins.

**Success:** agents cannot edit the same node concurrently (reservations); handoff is explicit in the
timeline; the user can pause one without the other.

### 5.5 Export to code

Bima opens the file, selects the login frame, chooses **Export → React + Tailwind**, previews the generated
component tree, downloads a ZIP with `LoginCard.tsx`, `tokens.css`, assets, and a README. The code compiles
with `tsc --noEmit` and renders within 2% visual diff of the PNG at the same viewport.

---

## 6. Competitive Frame

| Capability | Figma | Prompt-to-UI tools | Code-first builders | **Kanvas** |
|---|---|---|---|---|
| Professional canvas editing | Yes | No | Partial | **Yes** |
| Real design systems (tokens, variants, libraries) | Yes | No | Partial | **Yes** |
| AI generates inside an editable file | Limited/bolt-on | Generates its own artifact | Code-first | **Yes — same graph** |
| Visible agent cursor & plan | No | No | No | **Yes** |
| Multi-agent collaboration with humans | No | No | No | **Yes** |
| Per-agent undo & audit | No | No | No | **Yes** |
| Production code export | Plugins | Yes (whole app) | Yes | **Yes (component-level)** |

Positioning sentence: **"The design tool agents can actually use — and you can actually edit."**

We do not compete on model quality; we compete on **the quality of the interface between models and design
work**: context compression, typed tools, deterministic layout, and trust UX.

---

## 7. Scope

### 7.1 MVP (must ship; see `09-implementation-plan.md` for milestones)

| Area | MVP contents |
|---|---|
| Canvas | Infinite canvas, pan/zoom, selection (click/box/lasso), drag, snapping, rulers, guides, grids, frames, groups, z-order, duplicate, copy/paste, undo/redo, rotate, resize, lock, hide, rename, search, zoom-to-selection |
| Shapes | Rectangle, rounded rect, ellipse, line, arrow, triangle, diamond, polygon, star, frame, slice, vector/pen, freeform |
| Text | Single/multi-line, family/style/size/line-height/letter-spacing/align/color/case/decoration, auto-height/fixed/fill-width, rich ranges, placeholder styles |
| Auto layout | H/V, gap, padding, alignment, distribution, wrap, nested, fill/hug/fixed, min/max, absolute children |
| Components | Create/instantiate/detach/swap, overrides, nested, variant sets, boolean/text/instance-swap properties, internal library publish |
| Design system | Variables/collections/modes, themes (light/dark), color/space/radius/shadow/typography tokens, binding UI, token panel |
| Assets | Upload image/SVG, library grid, search, drag to canvas, replace, dedupe, optimization |
| Layers/pages | Nested tree, search, visibility/lock, reorder, multi-select, indicators, multi-page files |
| Prototyping | Frame links, click/hover/drag triggers, navigate/back/overlay actions, dissolve/smart-animate, preview mode |
| Collaboration | Live cursors, presence, comments with threads/mentions/resolve, version history, activity log |
| AI | Agent connect (built-in + external via MCP/local bridge), standby cursor, prompt bar, plan, visible execution, action log, stop/pause/takeover, confirmation gates, per-run undo, 12 core tools |
| Export | PNG/JPG/SVG/PDF, scale presets, batch ZIP, JSON design structure, tokens (JSON/CSS), basic React+Tailwind and HTML/CSS |

### 7.2 V1 (post-MVP, 3–6 months)

Team libraries with semver updates; variant interaction states; token migration tooling; Flutter/React
Native export; multi-agent orchestration UI; AI accessibility/motion reviewers; responsive variant
generation; plugin API; comments on prototype; offline mode; Figma import (via SVG + JSON best-effort).

### 7.3 Full vision (12–24 months)

Design-system publishing marketplace; runtime AI prototypes (`AI_GENERATE` actions); AI design reviewer
that files comments; design analytics ("where do users drop off in this flow?"); plugin ecosystem; native
mobile apps; enterprise governance (SSO/SCIM, retention, DLP); code round-trip (edit code → update design).

### 7.4 Non-goals

- Photorealistic image generation as the primary product.
- Hidden AI edits without user visibility.
- Chat-only design with no canvas.
- 3D, video editing, or motion-graphics authoring.
- Enterprise procurement complexity in MVP (SSO, SCIM, on-prem).
- Owning deployment of user apps (we export; we do not host their production app).
- Being a general-purpose whiteboard (FigJam parity is not a goal; we support basic sections/arrows only).

---

## 8. Success Metrics

### 8.1 North star

**Weekly successful agent-assisted design sessions per active team** — a session where the user accepts the
result (runs an export, shares, or continues editing without reverting the agent's run).

### 8.2 Product metrics & targets (first 6 months post-GA)

| Metric | Definition | Target |
|---|---|---|
| Activation | New user creates a frame and completes one agent run in first session | ≥ 55% |
| Time to first meaningful design | Signup → first frame exported or shared | median < 8 min |
| Agent task success | Runs ending `COMPLETED` with no error and no user undo within 10 min | ≥ 80% (MVP tool set) |
| Agent revert rate | Runs reverted via "Undo run" within 10 min | ≤ 12% |
| Token compliance | Themed fills/strokes bound to variables in agent-created nodes | ≥ 90% |
| Manual-edit retention | Users who edit agent output by hand within session | ≥ 60% (proves editability, not a toy) |
| D7 / D30 retention (teams) | Team returns and edits | 35% / 18% |
| Code export adoption | Teams exporting code weekly | ≥ 25% of active teams |
| p95 interaction latency | Pan/zoom/drag frame time | ≤ 16.7ms (60fps), p95 ≤ 24ms |
| Agent stop latency | Stop click → no further applied commands | p95 ≤ 250ms |
| Crash-free sessions | | ≥ 99.5% |

### 8.3 Quality bars (gates, not aspirations)

- **Editing parity test:** the 25-task Figma-parity checklist in `09-implementation-plan.md` §7 passes on
  the release candidate, keyboard-only.
- **Agent eval suite:** 40 golden prompts; ≥ 85% structural pass; no destructive-op violations across 200
  adversarial runs (prompt injection fixtures included).
- **Export fidelity:** React+Tailwind output for the 12 fixture designs compiles and renders within 2%
  pixel diff; SVG export within 0.5%.
- **Accessibility:** axe clean on editor chrome and prototype player; keyboard-complete for core flows.

---

## 9. Constraints & Assumptions

| # | Assumption | If wrong |
|---|---|---|
| A1 | Browser canvas (Konva/WebGL) can hold 60fps at 10k nodes/page with culling | Invest earlier in WebGL renderer; cap page node budget |
| A2 | LLM tool-calling reliability is sufficient with typed tools + preconditions | Add server-side plan compiler that validates and repairs tool calls |
| A3 | Users tolerate 30–90s agent runs when progress is visible | Add "fast mode" (silent execution + summary), optimize step batching |
| A4 | Teams will accept AI edits if undo/audit are strong | Increase approval gating; add review-before-apply mode |
| A5 | Liveblocks + Supabase meet latency/cost needs at beta scale | Swap to self-hosted Yjs + Postgres/Redis behind `SyncProvider` interface |
| A6 | External agent CLIs (Claude Code, Codex) can connect via local bridge/MCP | Ship built-in orchestrator as the only agent path initially |

---

## 10. Requirement Traceability

| Brief section | Where specified |
|---|---|
| §1–5 Vision, goals, users, use cases | This document |
| §6.1–6.4 Canvas, shapes, text, auto layout | `03-feature-spec-core.md` |
| §6.5–6.12 Components, design system, assets, layers, pages, prototyping, collaboration, import/export | `02-data-model.md`, `04-feature-spec-systems.md`, `08-export-and-codegen.md` |
| §7 AI agent layer | `05-ai-agent-layer.md` |
| §8 UX requirements | `01-ux-architecture.md` |
| §9–10 Data model & command model | `02-data-model.md`, `07-state-model.md` |
| §11 Agent workflow example | `05-ai-agent-layer.md` §9 |
| §12 Realtime expectations | `07-state-model.md` §6 |
| §13 Accessibility | `01-ux-architecture.md` §12, `05-ai-agent-layer.md` §10 |
| §14 Performance | `06-architecture.md` §9 |
| §15–16 Stack & architecture | `06-architecture.md` |
| §17–18 MVP & full vision | This document §7, `09-implementation-plan.md` |
| §19–21 Non-goals, acceptance, example prompt | This document §7.4, §8.3; `05-ai-agent-layer.md` §9 |
| §22–24 Definition & output requirements | `README.md`, all documents |

---

## 11. Decision Log (vision-level)

| ID | Decision | Rationale | Reversible? |
|---|---|---|---|
| D-001 | Build a real editor, not a generator | Editing is the durable job; generation is a commodity | No |
| D-002 | Agents use the same command layer as humans | Attribution, undo, conflict handling come for free | No |
| D-003 | Visible cursor by default, instant mode optional | Trust is the adoption bottleneck | Yes (per-user setting) |
| D-004 | Token-first agent behavior | Design-system fidelity is the professional differentiator | No |
| D-005 | Multi-provider agent support from day one of the protocol | Avoid single-vendor dependency; users already have preferred CLIs | No |
| D-006 | Desktop-web first, mobile viewer later | Canvas editing on touch is a different product problem | Yes |
| D-007 | Supabase + Liveblocks for MVP, Yjs-compatible abstraction | Speed to market with a credible exit path | Yes |
| D-008 | Component-level code export, not whole-app generation | Matches handoff reality; avoids competing with app builders | Yes |
| D-009 | No `.fig` import in MVP | Parsing undocumented format is a time sink; SVG/JSON cover most value | Yes |
| D-010 | Confirmation gates for destructive ops, never silent | Trust principle P6 | No |

---

## 12. Glossary Quick Reference

| Term | Meaning |
|---|---|
| **Run** | One prompt-to-completion agent execution, with plan, actions, summary, cost |
| **Plan / Step** | Visible ordered intent; steps can be approved, skipped, rerun |
| **Action** | A recorded agent effect with rationale, tool, commands, result |
| **Playback** | The choreographed cursor animation that presents an action |
| **Takeover** | A human assuming control of a node an agent is editing; agent yields |
| **Reservation** | A short-lived soft lock on a subtree during an agent step |
| **Guardrail** | Policy limiting what an agent may do (scopes, budgets, confirmations) |
| **Handle** | Run-scoped short node alias used in LLM context (`h_F1`) |
| **Envelope** | Attributable wrapper around one or more design commands |
| **Pulse** | Visual highlight marking nodes changed by an agent action |
| **Frozen prompt bar** | Always-visible, pinned command input at the bottom of the editor |
