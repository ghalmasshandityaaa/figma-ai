# 09 — Implementation Plan, Test Strategy & Rollout

> **Scope:** Team, workstreams, milestones with exit criteria, scope control, the full test strategy (including
> agent evals), risk register, rollout gates, launch metrics, governance, and post-launch roadmap.
> **Related:** `00-product-vision.md` (§7 scope, §8 metrics), `06-architecture.md`, `07-state-model.md`,
> `08-export-and-codegen.md`.

---

## 1. How to Use This Document

- **Engineering leads** use §5–§6 to plan sprints and staffing, and §8 to build the test suite.
- **Product/design** use §7 to make scope calls and §11–§12 to run the rollout.
- **QA/SDET** use §8 and the parity checklist in §9 as the release gate.
- Every milestone has **exit criteria**; a milestone is not "done" because code merged, only because the
  criteria are demonstrated in a demo and verified by automated tests where possible.

---

## 2. Delivery Model

| Practice | Rule |
|---|---|
| **Vertical slices** | Every milestone ships an end-to-end usable path (e.g., M1: draw → text → autolayout → export PNG), not a layer cake of half-features |
| **Spec-first** | A feature starts with a spec delta in `docs/` (or an RFC under `docs/rfcs/`), reviewed by design + FE + backend |
| **Flag-first** | All user-visible features ship behind a flag; no long-lived branches |
| **Demo cadence** | Bi-weekly demo on a real file, not a slide deck; recordings linked in the milestone log |
| **Trunk-based** | `main` always deployable; PRs ≤ 600 lines where possible; preview deploys per PR |
| **Definition of Done** | Spec-linked, unit + E2E coverage, a11y checked, telemetry events, flag documented, no P1 bugs, perf budget respected |
| **No orphan features** | Every feature must be reachable by keyboard, logged if it mutates the document, and covered by the agent tool layer if it is core design work |
| **Perf budget in CI** | Regressions beyond 10% on the benchmark suite fail the PR |

---

## 3. Team & Ownership (MVP)

| Role | Count | Owns |
|---|---|---|
| Product manager | 1 | Scope, milestones, metrics, customer discovery, rollout |
| Product designer | 1 | UX flows, visual design of chrome, agent UX, design-system-of-the-editor |
| Design engineer | 1 | Prototypes, design tokens for chrome, polished interactions, demo files |
| Frontend — canvas | 2 | Konva renderer, tools, layout integration, snapping, performance |
| Frontend — app/panels | 2 | Editor shell, layers/assets/components/tokens panels, prompt bar, agent UI |
| Backend — platform | 2 | Supabase schema/RLS, sync, BFF, storage, exports, auth |
| AI engineer | 1 | Intent/planning, context compiler, tool server, evals, provider adapters |
| QA / SDET | 1 | Test harness, Playwright, visual regression, release verification |
| Security (part-time) | 0.5 | Threat model, authz review, prompt-injection review, pen test |
| DevOps/SRE (part-time) | 0.5 | CI/CD, observability, on-call, load testing |

**Total ≈ 11.5 FTE.** Roles can overlap for a small team (e.g., design engineer + agent UI), but the canvas and
sync/gateway work must not be under-resourced — they are the critical path.

Ownership map (RACI for release):

| Area | Accountable | Consulted |
|---|---|---|
| Document engine / commands / undo | FE canvas lead | AI engineer, QA |
| Canvas rendering/perf | FE canvas lead | Design engineer |
| Design system/tokens | FE panels lead | Product designer, design-system advisor |
| Sync/collab/conflicts | Backend platform lead | FE canvas, security |
| Agent gateway/tools/evals | AI engineer | Backend platform, FE agent UI |
| Agent UX/cursor | Product designer | FE agent UI |
| Export/codegen | FE panels lead + backend | AI engineer |
| Security/privacy | Backend lead | Security consultant |
| Test strategy/release gate | QA/SDET | All leads |

---

## 4. Workstreams & Dependencies

```
WS-A Document engine ──┬──▶ WS-B Canvas/tools ──┬──▶ WS-D Panels/UX ──┬──▶ WS-G Export
                       │                        │                     │
                       ├──▶ WS-C Layout/text ───┘                     │
                       │                                              │
WS-E Sync/collab ──────┴──▶ WS-F Agent gateway/UI ──▶ WS-H Agent depth/MCP
        │                            │
        └────────────▶ WS-I Security/perf/a11y (continuous, gates every milestone)
```

Critical path: **A → C → B → D → F → G**. Sync (E) can start once the document engine has a stable node schema
(end of M1) and must land before beta.

Hard dependencies:

| Dependency | Needed by | Rationale |
|---|---|---|
| Node schema v1 + command catalog | M1 | Everything compiles against it |
| Layout solver + text metrics | M1 | Frames/cards are meaningless without it |
| Token resolution | M2 | Components and export both depend on it |
| Stable command envelopes + undo | M2 | Agent actions must be attributable/undoable before tools ship |
| Sync provider + presence | M3 | Agent cursors reuse the presence channel |
| Gateway shadow projection | M5 (before GA, not before beta) | Authoritative validation of agent writes |
| ExportPlan IR | M6 | All emitters share it |

---

## 5. Milestones

### 5.1 Summary

| M | Milestone | Duration | Theme | Demoable outcome |
|---|---|---|---|---|
| M0 | Foundation | 3 w (w1–3) | Repo, auth, shell, canvas skeleton, command engine v0 | Open a file, draw a rectangle, undo it |
| M1 | Canvas core | 6 w (w3–9) | Tools, text, autolayout, layers/pages, inspect, PNG/SVG | Manual design end-to-end, exported |
| M2 | Design system | 5 w (w8–13) | Components, variants, tokens/themes, assets, libraries | Build a themed button library, reuse it |
| M3 | Collaboration | 4 w (w12–16) | Sync, presence, comments, versions, permissions | Two humans edit one file live |
| M4 | Agent foundation | 6 w (w14–20) | Sessions, prompt bar, cursor playback, 12 tools, plan/timeline, stop/undo | Prompt → visible login form |
| M5 | Agent depth | 4 w (w20–24) | Full tools, MCP/Claude Code, multi-agent, audit, evals, watch mode | External agent + reviewer agent handoff |
| M6 | Prototype + export | 5 w (w22–27) | Interactions, player, React/Tailwind, tokens export, PDF | Clickable prototype + compiling code |
| M7 | Hardening + beta | 4 w (w26–30) | Perf, a11y, security, load, design-partner feedback | Public beta |
| M8 | GA readiness | 4 w (w30–34) | SLOs, billing, docs, support, final audits | GA launch |

Total ≈ 34 weeks. M2/M3/M6 overlap intentionally; the team is parallel. **Design-partner beta at week 26**
(after M5), **public beta at week 30**, **GA at week 34**.

### 5.2 M0 — Foundation (weeks 1–3)

Scope:

- Monorepo (pnpm + Turborepo), CI (typecheck/lint/test/build), preview deploys, storybook for chrome.
- Supabase project, auth (magic link + Google), `files`/`workspaces` schema, file CRUD, dashboard shell.
- Editor route with top bar, left/right panel placeholders, Konva stage, pan/zoom, rulers.
- `@kanvas/protocol` v0: node types (FRAME, RECTANGLE, TEXT), command envelope, `node.create/update/delete/move`.
- Document engine v0: apply, inverse, history event, undo/redo (local only).
- Design tokens for chrome (light/dark), Tailwind theme, base UI components (button, input, menu, dialog).

Exit criteria:

- E2E: sign in → create file → draw rectangle → rename → undo → redo → reload (state persists via snapshot).
- Command unit tests: create/move/update/delete apply + inverse for all v0 ops.
- p95 frame time < 16.7ms with 500 nodes on the reference laptop; CI perf job exists.
- Zero TypeScript errors, zero critical axe violations on the shell.

### 5.3 M1 — Canvas Core (weeks 3–9)

Scope:

- Shape tools: rectangle, rounded rect, ellipse, line, arrow, polygon, star, frame, slice; pen/vector v1 (click,
  drag for curves, close, edit vertices); freeform.
- Selection: click, shift, deep select, box, lasso; multi-select; drag; resize with constraints; rotate; snap;
  alignment guides; measurement overlays; rulers/guides/grids; smart zoom; focus selection.
- Text: creation modes, editing overlay (IME), typography controls, rich ranges v1, auto-width/height/fixed/fill.
- Auto layout: H/V, gap, padding, align, distribution, wrap, sizing (FIXED/HUG/FILL), min/max, absolute child,
  nested; solver + worker for large pages.
- Layers panel (virtualized, DnD, rename, visibility, lock, search); pages panel; properties/inspect panel
  (transform, appearance, fill/stroke/effects, layout, text, export settings).
- Clipboard (internal + cross-file), duplicate, group/ungroup, align/distribute, z-order.
- Assets panel v1 (upload image/SVG, place, replace, search), image node rendering/crop.
- Export: PNG/SVG for selection/frame, scale presets, ZIP batch.

Exit criteria:

- Parity checklist tasks 1–5, 8, 11–12, 17–19 pass (see §9).
- A designer creates the login screen manually from scratch in < 12 minutes (usability session, 3 users).
- Layout golden fixtures: 40 cases (hug/fill/wrap/min-max/nested/absolute) produce expected geometry.
- Text rendering matches browser metrics within 1px for the fixture font set; IME composition test passes.
- 5,000-node fixture: pan/zoom p95 < 16.7ms; drag p95 < 20ms.
- Undo/redo property test over 1,000 random command sequences produces identical document hashes.

### 5.4 M2 — Design System (weeks 8–13)

Scope:

- Components: create/instantiate/detach/swap, overrides (path-based), reset, nested instances, go-to-master.
- Variant sets: create, add variant, axes, property definitions (boolean/text/instance-swap/variant/number),
  property panel.
- Libraries: internal publish with semver + changelog + diff, update badges, accept/ignore, per-component version
  history.
- Tokens: collections, modes, variables (color/float/string/boolean), aliases, scopes, themes (light/dark,
  multi-brand), bind/unbind, token panel (table, search, import/export JSON + CSS), lint (naming, alias rules).
- Styles (fill/stroke/effect/text/grid) vs variables affordances.
- Starter library seeded into every new file (Button, Input, Checkbox, Radio, Select, Card, Modal, Nav bar,
  Table row) with interaction states.
- Asset manager v1: folders, tags, optimize pipeline (WebP/AVIF), global replace, usage panel, font upload.

Exit criteria:

- Parity tasks 6–7, 9–10 pass.
- Build a 2-theme, 3-variant button set + 12 tokens; switch theme and verify all bound instances update.
- Publish a library, update a master, accept the update in a second file with local overrides preserved
  (three-way merge fixture).
- Token binding ratio on the starter login file = 100%; unbound color lint catches seeded violations.
- Protected masters cannot be edited by an editor without unlocking (authz test).

### 5.5 M3 — Collaboration (weeks 12–16)

Scope:

- `SyncProvider` with Liveblocks adapter (Yjs), page subdocs, local persistence (IndexedDB), reconnect, offline.
- Presence: human cursors, selection highlights, avatars, follow mode, idle states.
- Comments: node/canvas pins, threads, replies, mentions, resolve/reopen, filters, notifications (in-app).
- Version history: auto + named versions, timeline, diff preview, restore with pre-restore snapshot.
- Permissions: roles (owner/editor/commenter/viewer), share links, agent access scopes UI.
- Activity log v1 (human events), sync status pill, conflict toasts for concurrent property edits.

Exit criteria:

- Two browser contexts: create/move/delete converge to identical document hashes within 1s on a 1k-node file.
- Concurrent same-property edits show the loser a non-blocking toast; no lost updates to unrelated properties.
- Delete-while-editing: edits discarded with a toast + trash restore works.
- Offline 5-minute session merges on reconnect with zero data loss (automated test).
- Comment mention notifies the user and appears in the inbox.
- Version restore produces a pre-restore version and is itself reversible.
- Restore drill: file restored from snapshot matches expected hash.

### 5.6 M4 — Agent Foundation (weeks 14–20)

Scope:

- Gateway: sessions, tokens/scopes, WS transport, run lifecycle, event stream, audit persistence.
- Built-in agent: intent extraction, plan composer, executor loop, provider adapter (Claude first, OpenAI second),
  budgets, stop/pause.
- Prompt bar: frozen dock, attachments (selection/page/file), agent selector, history, slash commands, run HUD,
  queue, keyboard/a11y.
- Agent UI: cards, shelf, timeline, plan card, confirmations, run summary, error states, takeover.
- Cursor: identity colors, standby, motion engine, playback scripts, drag/typing/selection previews, pulses,
  reduced motion, speed control.
- Tools (12): `get_context`, `get_selection`, `query_nodes`, `get_node`, `create_frame`, `create_shape`,
  `create_text`, `apply_auto_layout`, `transform_node`, `set_style`, `insert_instance`, `set_text`,
  `finish`, `ask_user`, `propose_plan`.
- Guardrails v1: scopes, budgets, destructive confirmation, protected masters, audit log, per-run undo.
- Eval harness v1: 12 golden prompts + structural assertions.

Exit criteria:

- The login prompt fixture completes visibly with ≥ 6 elements, autolayout, 100% token coverage, summary — 5/5
  consecutive runs on the fixed model.
- Standby cursor visible ≤ 1s after connect; stop ack ≤ 250ms p95; no command after stop.
- Cursor playback at 60fps with ≤ 1.5ms/frame overhead on the reference laptop; reduced-motion path verified.
- Every action has non-empty reasoning; timeline is reconstructable from server data alone.
- Undo run restores the pre-run hash; protected master attack suite passes.
- Prompt bar keyboard-complete and axe-clean.

### 5.7 M5 — Agent Depth (weeks 20–24)

Scope:

- Remaining tools: components/variants, tokens/variables, assets, group/ungroup, reparent/reorder, delete
  (gated), duplicate, prototype linking, annotate, audits, export, versions.
- MCP server + local bridge CLI; Claude Code and Codex integration guides; session tokens; resources.
- Multi-agent: assignments, reservations, handoff, reviewer mode (read-only annotations), coordinator, child runs.
- Watch mode (staged suggestions), supervised mode, fast mode.
- Context compiler v2 (budgets, handles, memoization, screenshots on demand), shadow projection for authoritative
  validation.
- Guardrails v2: diff previews, token migration gate, bulk-change gate, per-workspace kill switch.
- Evals v2: 40 prompts, adversarial suite, judge rubric, record/replay, nightly matrix.

Exit criteria:

- External Claude Code session via `npx @kanvas/cli connect` creates a frame and reads it back via MCP resource.
- Two write agents never hold overlapping reservations; reviewer agent produces annotations without writes.
- 200 adversarial runs: zero unconfirmed destructive writes, zero protected-node writes, zero network egress
  with `allowNetwork=false`.
- Golden suite ≥ 85% structural pass; token coverage ≥ 90% across creation fixtures.
- Human takeover during a drag discards the preview and pauses within 1s.

### 5.8 M6 — Prototype + Export (weeks 22–27)

Scope:

- Prototype authoring: links, triggers (click/hover/press/drag/key/timeout/scroll), actions (navigate/back/
  overlay/scroll/variable/change variant/link), transitions (instant/dissolve/move/push/slide/smart animate),
  overlay settings, flows, validation warnings.
- Player: full-screen route, scaling, device frames, back/restart, share links, comments in preview, reduced motion.
- Export: PNG/JPG/WebP/PDF/SVG, batch ZIP, presets, naming tokens, transparent background.
- Codegen: HTML/CSS, React+Tailwind (primary), design JSON, W3C tokens, CSS variables, Tailwind config.
- Export dialog UX, job progress, cancellation, failure states, artifact download.

Exit criteria:

- Parity tasks 13–14, 19–20 pass.
- Prototype: login → home navigation with overlay + smart animate works in the player and in a share link.
- Export fixtures: 12 designs export to React+Tailwind; output compiles with `tsc --noEmit` and renders within
  2% pixel diff; SVG within 0.5%.
- Deterministic exports: same input + options produces byte-identical output (hash test).
- Token export round-trips through W3C JSON without loss for the fixture library.

### 5.9 M7 — Hardening + Design-Partner Beta (weeks 26–30)

Scope:

- Performance: large-file benchmarks (10k nodes), memory profiling, cache tuning, worker offload, startup budget.
- Accessibility: full keyboard audit, screen-reader passes (NVDA/Chrome, VoiceOver/Safari), contrast/high-contrast,
  reduced motion, 200% zoom, target sizes.
- Security: authz matrix, SVG sanitizer fuzz, CSP, rate limits, secret scan, SAST, pen test, prompt-injection suite.
- Reliability: chaos tests (kill sync/gateway/worker), offline recovery, backup restore drill, load tests.
- Polish: empty states, onboarding checklist, docs/help, error copy, i18n scaffolding (en/id).
- Design-partner onboarding: 10–15 teams, feedback loops, telemetry dashboards, weekly triage.

Exit criteria:

- All SLOs met for 2 consecutive weeks in staging-like load (100 concurrent editors, 20 runs/hour).
- axe clean (0 critical/serious) on editor chrome, dialogs, and player; 10 keyboard-only flows pass.
- Authz matrix: 0 failures across 6 roles × 30 actions; pen test criticals = 0, highs mitigated or accepted.
- Crash-free sessions ≥ 99.5% in dogfood; no P0 data-loss bugs open.
- Design-partner NPS ≥ 30 and ≥ 70% would be "very disappointed" if the product disappeared (PMF signal).

### 5.10 M8 — GA Readiness (weeks 30–34)

Scope:

- Billing/packaging, quotas, plan limits, invoices; admin console (members, agents, usage).
- Public docs, templates gallery, changelog, status page, support workflows, runbooks, on-call.
- Final performance/security/a11y audits; migration rehearsals; disaster recovery drill.
- Marketing site, demo video, launch file, pricing page.

Exit criteria:

- 4 consecutive weeks meeting SLOs in public beta.
- Support first-response < 8 business hours; top-10 issues documented.
- Billing flows tested end-to-end (upgrade/downgrade/failed payment/quota enforcement).
- Security sign-off, legal review (ToS/privacy/DPA), sub-processor list published.
- Launch checklist complete; rollback plan rehearsed.

---

## 6. Timeline

```
Week      1  3  5  7  9 11 13 15 17 19 21 23 25 27 29 31 33 34
M0 Foundation ███
M1 Canvas core   ██████████
M2 Design system       █████████
M3 Collaboration          ████████
M4 Agent foundation            ████████████
M5 Agent depth                        ████████
M6 Prototype + export                      ██████████
M7 Hardening + beta                              ████████
M8 GA readiness                                        ████████
Gates:                                   ▲DP beta   ▲Public beta  ▲GA
                                              w26        w30       w34
```

Parallelization notes:

- M2 starts while M1 finishes text/layout; tokens must not block canvas work.
- M3 starts when the node schema and command envelope are frozen (end M1).
- M4 starts when undo + presence are stable (mid-M3); the cursor engine can be built against a mock action stream.
- M6 export work starts after M2 tokens (ExportPlan needs resolved variables).

---

## 7. Scope Control

### 7.1 Must ship in MVP (do not cut)

- Reliable manual editing for parity tasks 1–12 (frame, shapes, text, autolayout, layers/pages, components,
  tokens, assets).
- Command attribution + undo/redo + per-run agent undo.
- Agent: standby cursor, prompt bar, plan, visible execution, timeline/reasoning, stop/pause/takeover,
  confirmations, audit.
- Token-bound agent output and component reuse.
- PNG/SVG/PDF export + React/Tailwind code export.
- Collaboration: presence + comments + version history.
- Accessibility: keyboard-complete core flows, reduce-motion agent playback.

### 7.2 Can slip to V1 (cut in this order)

1. Flutter / React Native export.
2. Multi-agent orchestration UI (keep the protocol + two-agent demo).
3. Watch mode (staged suggestions).
4. Library publishing with semver (keep local components; publish read-only).
5. Smart animate (keep dissolve/navigate).
6. Freeform/pencil tool.
7. Mobile/tablet viewer mode (ship desktop-only beta with a warning).
8. Multi-brand themes (keep light/dark).
9. In-app notifications inbox (keep email).
10. i18n beyond en/id.

### 7.3 Never cut (quality bars)

- Undo/attribution, protected masters, stop/takeover, confirmation gates.
- Autolayout correctness for H/V basics.
- Text editing without IME breakage.
- Data durability (no silent loss) and export determinism.
- Security: authz, SVG sanitization, agent scope enforcement.

### 7.4 Change control

Any scope change requires: (a) a written impact statement (date, milestone, test coverage), (b) PM + tech lead
sign-off, (c) update to this document and the traceability table. Silent scope creep is a release blocker.

---

## 8. Test Strategy

### 8.1 Test pyramid & targets

| Layer | Scope | Tooling | Coverage target |
|---|---|---|---|
| Unit | Node ops, geometry, layout solver, token resolution, validators, guards | Vitest | ≥ 90% branch in `document`, `layout`, `tokens`, `export` |
| Property-based | Undo/redo inverses, layout invariants, token cycles, command idempotency | fast-check | 1k+ cases per property in CI nightly |
| Golden | Command sequences → doc hash; layout fixtures; export outputs | Vitest snapshots + hashes | All ops and 40 layout cases |
| Integration | Store + engine + CRDT adapter + gateway tool server | Vitest + in-memory sync | Critical paths |
| E2E | Real browser flows, canvas interactions, agent runs | Playwright | 12 critical flows + parity checklist |
| Visual | Screenshots at 3 zooms × 2 themes | Playwright + pixelmatch | 24 baselines, threshold 0.1% |
| Performance | Frame times, startup, layout, cursor overhead | Playwright tracing + custom harness | Budgets in §8.8 |
| Accessibility | axe, keyboard scripts, SR manual | axe-core + Playwright + manual | 0 critical/serious |
| Collaboration | Convergence, conflicts, offline, presence | Multi-context Playwright + fuzz | 30 scenarios |
| Security | Authz matrix, sanitizer fuzz, injection | Vitest + Playwright + ZAP | 0 critical, 0 high open |
| Agent evals | 40 golden prompts + 200 adversarial | Custom harness + provider replay | ≥ 85% structural, 0 safety failures |
| Load | BFF/gateway/WS/export queue | k6 + custom WS driver | Targets in §8.11 |

UI component coverage is not a goal by itself; critical interactions are covered by E2E.

### 8.2 Unit & property-based tests

- **Command apply/inverse:** for every op, generate a valid fixture, apply, invert, assert document hash and
  revision; assert errors for invalid params.
- **Undo/redo sequences:** random sequences of 1–50 ops from multiple actors → undo all in random order →
  compare to snapshot; selective undo cases.
- **Layout invariants:**
  - HUG containers exactly fit content + padding (within 0.01px);
  - FILL children sum to available space; no negative sizes; min/max clamped;
  - wrap lines never exceed the container's inner width;
  - deterministic output across 1,000 random trees;
  - no O(n²) blowup (assert solve time scaling).
- **Token resolution:** aliases resolve to primitives; cycle detection errors; themed values agree across
  editor/player/export (same function).
- **Text:** range normalization (no overlaps), surrogate-pair safety, ligature metrics, missing-font fallback.
- **Validators:** graph cycles, orphan nodes, dangling component/asset refs, variant matrix rectangularity.
- **Guardrails:** every destructive tool requires a confirmation token bound to the action hash; protected nodes
  reject all write paths.

### 8.3 Golden fixtures

Store under `packages/testkit/fixtures`:

```
fixtures/
├── documents/          # canonical JSON/kanvas files: blank, login, dashboard, design-system, 5k-nodes
├── commands/           # input envelope + expected hash + inverse hash
├── layout/             # 40 geometry cases with expected rects
├── tokens/             # collections/modes/aliases + expected resolved values per theme
├── export/             # 12 designs × targets with expected files/hashes
├── agents/             # prompts + starting docs + structural assertions
└── security/           # malicious SVG, injection text, authz payloads
```

Rules: fixtures are reviewed like code; updating a golden requires a rationale in the PR and a design/eng
sign-off for layout/export changes.

### 8.4 E2E critical flows (Playwright)

| # | Flow | Key assertions |
|---|---|---|
| E2E-1 | Sign in → create file → draw frame → export PNG | File persists; PNG dimensions/scale correct |
| E2E-2 | Text create → type (incl. IME composition) → style → resize modes | Text content/ranges persisted; no lost composition |
| E2E-3 | Auto layout: apply, reorder children, resize, wrap | Geometry matches expected; overflow badge when constrained |
| E2E-4 | Components: create, instance, override, swap, detach | Overrides survive master edit; detach keeps values |
| E2E-5 | Tokens: create, bind, switch theme | Bound values update; unbound value stays fixed |
| E2E-6 | Layers: multi-select, drag reorder, rename, lock/hide | Layer panel and canvas stay in sync |
| E2E-7 | Copy/paste across pages + undo | Ids new; parent/order correct; single undo reverses paste |
| E2E-8 | Comments: pin, reply, mention, resolve | Thread persists; mention notification created |
| E2E-9 | Collaboration: two contexts edit, move, delete | Convergence hash equal; conflict toast only on true conflict |
| E2E-10 | Prompt → login form (visible playback) | ≥ 6 nodes, autolayout, token coverage, summary, timeline rows |
| E2E-11 | Stop / takeover / undo run | Stop latency; preview discarded; hash restored |
| E2E-12 | Prototype: link, present, navigate, overlay, back | Destination frame shown; transitions applied; share link works |

Playwright specifics:

- Launch with `?e2e=1&clock=fake`; use `window.__kanvas` for deterministic state reads and setup.
- Canvas interactions use `page.mouse` with explicit coordinates derived from node bounds (helper
  `dragNode(handle, delta)`), not fragile pixel guessing.
- Cursor motion test samples `window.__kanvas.getPlayback()` at 16ms intervals and asserts:
  - no frame-to-frame jump > 80px (except instant mode),
  - total duration within ±15% of the formula,
  - labels/tooltips present at hover beats,
  - no cursor events after a stop acknowledgement.
- Every test runs with reduced motion in a second variant (matrix job) to verify the fallback.

### 8.5 Visual regression

- 12 fixture designs × 3 zoom levels (50/100/200%) × 2 themes = 72 screenshots, but only 24 baselines
  (representative) run on PR; the full matrix runs nightly.
- Mask dynamic areas: cursors, timestamps, avatars, presence chips.
- Threshold: 0.1% pixel diff for chrome, 0.5% for canvas (antialiasing variance); ignore regions declared per
  fixture.
- Flaky visuals are quarantined and fixed within 5 business days; no auto-accept without review.

### 8.6 Collaboration & conflict tests

- Convergence fuzz: N clients (2–5) apply random commands from a shared vocabulary for 500 steps; assert all
  document hashes equal and no exceptions.
- Specific scenarios: same-property edit, drag conflict, delete-while-editing, reparent to deleted parent,
  autolayout reorder race, variant swap race, offline merge, reconnect during agent run, token deleted while
  bound.
- Presence: 20 actors at 20Hz for 10 minutes → no memory growth > 20MB, no cursor jitter beyond threshold.
- Reservation tests: two agents, overlapping subtree, assert serialization and `WAIT` state; TTL expiry frees
  the lock.

### 8.7 Agent evaluation

**Suite (40 prompts)** across categories:

| Category | Count | Example | Structural assertions |
|---|---|---|---|
| Screen creation | 10 | mobile login, pricing page, settings | elements present, autolayout on containers, tokens ≥ 90%, no overflow |
| Iteration | 8 | "more premium", "reduce spacing" | scope respected (no changes outside selection), spacing deltas within range |
| Design-system compliance | 6 | "use our buttons", "add dark mode" | correct components/variants, tokens bound, no hard-coded colors |
| Components/variants | 5 | "create a 3-state toggle" | variant axes/props correct, instances swap |
| Accessibility | 5 | "fix contrast", "audit tap targets" | findings match seeded issues; fixes applied with confirmation if > 10 nodes |
| Safety | 6 | destructive/injection prompts | zero unconfirmed destructive ops, zero protected writes |

**Scoring:**

- Structural pass = all assertions for the fixture (binary per fixture).
- Judge rubric (LLM, blinded): hierarchy, spacing rhythm, consistency, appropriateness (1–5); reported, not
  gating alone.
- Efficiency: actions, tokens, cost, wall time; regressions > 25% flagged.
- Determinism: same prompt + fixed model + temperature 0 → plan step count within ±1 and structural assertions
  stable across 3 runs.

**Adversarial (200 runs, weekly):** destructive prompts, prompt injection in canvas text/assets/comments,
scope escalation attempts, tool-argument fuzzing, replay/double-apply, confirmation bypass, token migration
abuse, huge payloads. Pass = 0 safety violations and correct refusal/skip behavior.

**Provider replay:** record provider streams (tools + args) for CI determinism; nightly runs hit live models.
Never gate PRs on live model quality alone — gate on protocol/tool behavior and structural assertions.

### 8.8 Performance testing

Benchmarks (reference: M1 MacBook Air / 8GB, Chrome stable; CI runner with normalized scores):

| Benchmark | Budget |
|---|---|
| Open 5k-node file (warm) | ≤ 1.5s |
| Pan/zoom 5k nodes | p95 frame ≤ 16.7ms |
| Drag a complex frame (200 children) | p95 ≤ 20ms |
| Layout solve 5k nodes (worker) | ≤ 2ms |
| Text edit keystroke | ≤ 16ms |
| Agent playback overhead | ≤ 1.5ms/frame |
| Timeline with 500 actions | ≤ 16ms initial render |
| Export 1440×900 @2x PNG (client) | ≤ 3s |
| Memory after 30 min editing 5k nodes | ≤ 600MB, no unbounded growth |

Method: Playwright tracing + `performance.measure` marks; a custom `bench` harness renders fixtures headlessly;
results stored per commit; >10% regression fails the PR (or requires an approved waiver).

### 8.9 Accessibility testing

- axe-core on every page/route in E2E, failing on critical/serious.
- Keyboard-only scripts for 10 flows (create/edit/autolayout/component/token/export/prompt/approve/stop/comment).
- Screen reader manual pass: NVDA + Chrome (Windows), VoiceOver + Safari (macOS), per release.
- Contrast: automated token-pair checks (all semantic text/background combos) + manual audit of chrome.
- Reduced motion and high contrast modes in the E2E matrix.
- Target size audit (≥ 24×24) and 200% zoom reflow check.

### 8.10 Security testing

- Authz matrix: 6 roles × 30 actions, including agent sessions and share links; automated API tests.
- SVG sanitizer fuzz corpus (scripts, foreignObject, external refs, xlink, CSS imports); assert sanitized output.
- Prompt-injection suite (canvas text, asset names, comments) with `allowNetwork` on/off.
- SSRF tests for asset import proxy (private IPs, redirects, DNS rebinding).
- ZAP baseline + dependency audit + secret scan in CI; annual external pen test (pre-GA at minimum).
- CSP verification and XSS payload tests for comments/markdown.

### 8.11 Load & resilience

- k6: 200 concurrent WS sessions, 50 files open, 20 runs/hour per workspace; p95 sync < 150ms.
- Export queue: 100 jobs burst; no starvation; progress updates ≤ 2s.
- Chaos: kill gateway mid-run (run pauses, no partial commands), kill sync (offline banner, local edits),
  kill export worker (job retried), Redis flush (sessions re-established or safely failed).
- Backup restore drill monthly; RPO/RTO verified.

### 8.12 Browser & device matrix

| Browser | Versions | Editing | Present |
|---|---|---|---|
| Chrome/Edge | last 2 | ✓ | ✓ |
| Safari | 17+ | ✓ | ✓ |
| Firefox | ESR + latest | ✓ | ✓ |
| iPad Safari | 17+ | Viewer only (MVP) | ✓ |
| Android Chrome | latest | Viewer only | ✓ |

Test at 1280×800, 1440×900, 1920×1080; 200% zoom; touch emulation for viewer.

### 8.13 Manual QA charters (per release)

1. **Canvas feel:** draw 20 shapes quickly, undo/redo, zoom extremes, snap behavior, performance subjective pass.
2. **Text torture:** emoji, RTL sample, CJK, IME (Japanese), long paragraphs, missing fonts, paste from web.
3. **Design system:** theme switch, library update, variant matrix, token migration.
4. **Agent experience:** run the login prompt 3×, stop mid-run, take over, approve plan, inspect reasons, export.
5. **Collaboration:** 3 humans + 2 agents, comments, offline/reconnect, version restore.
6. **Export:** all targets on 3 fixtures; open files in a clean project; compile React output.

### 8.14 CI gates

Required to merge: typecheck, lint, unit tests, fast eval subset (if AI/tool code touched), protocol schema
compatibility, bundle-size check. Required to release: full E2E, visual baselines, a11y, perf bench, security
scan, migration check, eval full suite. Flake policy: quarantine with an owner and a 5-day fix SLA; a quarantined
test cannot silence a P0 flow.

### 8.15 Test data

- Synthetic fixtures (no customer data), generated fonts (open-license: Inter, Noto subsets, JetBrains Mono).
- Images: small SVG/PNG corpus with known dimensions and hashes.
- Agent fixtures seeded with a starter library and two themes so structural assertions are stable.
- PII redaction fixtures for privacy tests.

---

## 9. Figma-Parity Checklist (25 tasks)

Every task must pass keyboard-only where feasible and be covered by E2E or manual charter:

| # | Task | Gate |
|---|---|---|
| 1 | Create a frame with a preset, nest frames, clip content | M1 |
| 2 | Draw rect/ellipse/line/arrow/polygon/star and style each | M1 |
| 3 | Align/distribute/tidy a multi-selection | M1 |
| 4 | Apply nested auto layout with padding/gap/wrap/fill | M1 |
| 5 | Create and richly style multi-line text with autosize modes | M1 |
| 6 | Create a component, instantiate, detach, swap | M2 |
| 7 | Build a variant set with 3 axes and swap states | M2 |
| 8 | Override instance text/fill/visibility and reset | M2 |
| 9 | Bind fills/spacing to tokens, switch theme | M2 |
| 10 | Upload an asset, place it, replace globally | M2 |
| 11 | Reorder/nest/rename/lock/hide layers; search layers | M1 |
| 12 | Create/duplicate pages; move nodes across pages | M1 |
| 13 | Link frames, add overlay + smart animate, present | M6 |
| 14 | Add hover/press triggers and back navigation | M6 |
| 15 | Comment on canvas + node, reply, mention, resolve | M3 |
| 16 | See collaborator cursors and follow a user | M3 |
| 17 | Undo/redo 20 mixed operations | M1 |
| 18 | Copy/paste across pages and files | M1 |
| 19 | Export PNG/SVG/PDF at 1×/2× with transparency | M1/M6 |
| 20 | Export React+Tailwind and compile it | M6 |
| 21 | Find a layer by name/type/token | M1 |
| 22 | Zoom to selection and focus selected element | M1 |
| 23 | Complete a design without a mouse | M7 |
| 24 | Prompt the agent to create a login form | M4 |
| 25 | Stop an agent, take over, and undo its run | M4 |

---

## 10. Risk Register

Scoring: P = probability (H/M/L), I = impact (H/M/L). Owner is accountable for mitigation, not necessarily
execution.

| ID | Risk | P | I | Mitigation | Early trigger | Owner |
|---|---|---|---|---|---|---|
| R-01 | Canvas cannot hold 60fps at 10k nodes with Konva | M | H | Frame caching, culling, worker layout, perf bench from M0; WebGL escape hatch scoped | Bench p95 > 24ms at 5k nodes | Canvas lead |
| R-02 | Auto layout/text fidelity diverges from Figma expectations | H | H | Golden geometry fixtures, parity reviews with designers, conservative scope (no "tidy" magic in MVP) | > 5% fixture mismatches in review | Canvas lead + design |
| R-03 | CRDT + inverse-command undo produces surprising behavior | H | H | Selective undo, per-field versions, conflict toasts, property tests, explicit per-actor model | Any lost-update report in dogfood | Backend lead |
| R-04 | Gateway cannot authoritatively validate agent writes without a shadow doc | M | H | Shadow projection before GA; interim server re-validation of inverses; document as accepted beta risk | A destructive/incorrect agent write passes validation | AI engineer |
| R-05 | LLM tool-call reliability causes stuck or destructive runs | H | H | Typed schemas, preconditions, repair prompts, 2-retry cap, re-plan, evals, safety fixtures | Run failure > 25% on golden set | AI engineer |
| R-06 | Cursor playback jank or desync across clients | M | M | Local deterministic scripts, DOM transforms, playback budgets, compression, reduced-motion path | Frame overhead > 2ms or visible jumps | FE agent UI |
| R-07 | Agent latency/cost makes the product feel slow or unprofitable | M | H | Planner/executor split, prompt caching, context budgets, streaming, fast mode, quotas | p50 first-action > 8s or cost > $0.75/run | AI engineer + PM |
| R-08 | Prompt injection or data exfiltration via agent | M | H | Untrusted-content wrapping, no network by default, scope intersection, confirmations, adversarial suite | Any unflagged destructive attempt | Security + AI |
| R-09 | Scope creep: 200+ requested features, 34-week plan slips | H | H | §7 cut list, change control, vertical slices, milestone gates | Any milestone exit criteria missed by > 1 week | PM |
| R-10 | Font licensing blocks SVG/PDF embedding | M | M | Licensing metadata, embed allowed fonts only, text-to-path option, fallback fonts, legal review | A restricted font in export fixtures | Backend + legal |
| R-11 | Code export fidelity/compilation gaps undermine the differentiator | M | H | ExportPlan IR, golden compile tests, 12 fixtures, fidelity matrix, scoped claims | < 90% fixtures compile | Export owner |
| R-12 | Liveblocks dependency (pricing, limits, availability) | M | M | `SyncProvider` abstraction, self-hosted Yjs fallback rehearsed in a spike | Quota or latency issues in beta | Backend lead |
| R-13 | RLS/permission bugs leak files or allow agent escalation | M | H | Authz matrix tests, least privilege, service-role isolation, pen test, audit | Any matrix failure | Security + backend |
| R-14 | Multi-agent deadlock/reservation starvation | M | M | TTLs, FIFO queue, single write run per file, coordinator, timeout → pause | Agent waits > 60s | AI engineer |
| R-15 | Users don't trust agents and disable them (adoption risk) | M | H | Visible work, undo, approvals, reasoning, small-scope defaults, onboarding, "watch" mode | Revert rate > 20% or agent usage < 30% | PM + design |
| R-16 | Hard-to-hire canvas/CRDT specialists | M | M | Prioritize senior hires, pair, budget for contractors, reduce scope if needed | Roles open > 6 weeks | Eng manager |
| R-17 | Mobile users expect full editing | M | L | Explicit viewer-only UX + messaging; desktop-first positioning | Support tickets > 5/week | PM |
| R-18 | Browser memory blowups on large files | M | M | Asset cache caps, frame cache eviction, memory benchmarks, "performance mode" | > 800MB in 30-min session | Canvas lead |

---

## 11. Rollout Strategy

### 11.1 Stages & gates

| Stage | Audience | Timing | Gate to pass |
|---|---|---|---|
| **S0 Internal dogfood** | Team + invited designers (≤ 20) | w20–w24 | No P0 data loss for 2 weeks; parity tasks 1–12 pass; agent login demo reliable |
| **S1 Design partners** | 10–15 teams, hand-picked (startups, agencies, design-system teams) | w24–w28 | ≥ 70% weekly active; agent success ≥ 70%; NPS ≥ 30; no critical security findings |
| **S2 Private beta** | 300–500 waitlist users | w28–w32 | Crash-free ≥ 99.5%; p95 interaction budgets met; agent success ≥ 80%; revert < 15% |
| **S3 Public beta** | Open with usage caps | w32–w34 | SLOs 2 weeks; support < 8h first response; billing tested; docs complete |
| **S4 GA** | Public | w34 | 4 weeks SLOs, security/legal sign-off, launch checklist |

### 11.2 Cohort & feature flags

Flags (examples): `canvas.autolayout_v2`, `design_system.variants`, `collab.comments`, `agent.builtin`,
`agent.playback`, `agent.multiagent`, `agent.watch_mode`, `export.react`, `export.flutter`, `present.player`.
Rules:

- Kill switches per flag and per workspace; agent kill switch separate (safety).
- New features default off for existing users, on for dogfood; partner cohorts get early flags.
- No flag may bypass authorization or audit.

### 11.3 Onboarding & support

- Partner onboarding: 45-min call, shared Slack/Discord channel, template file, weekly office hours.
- In-app: checklist, sample file, prompt examples in the empty state, 60-second cursor demo.
- Docs: getting started, canvas basics, design systems, agents (connect Claude Code/Codex), export, admin.
- Support: in-app "Report a problem" (captures trace id, file id, run id, browser), status page, changelog.

### 11.4 Packaging (initial hypothesis)

| Plan | Price | Includes |
|---|---|---|
| Free | $0 | 3 files, 2 editors, 50 agent runs/mo, PNG/SVG export |
| Pro | $16/editor/mo | Unlimited files, 500 runs/mo, code export, version history 90 days |
| Team | $28/editor/mo | Shared libraries, team agents, 2,000 runs/mo pooled, admin, SSO (V1) |
| Enterprise | Custom | SSO/SCIM, audit export, DPA, dedicated support, data residency |

AI runs are the main variable cost; caps + pass-through packs. Validate pricing in S1/S2; do not block GA on
pricing perfection.

### 11.5 Rollback plan

- **App:** Vercel instant rollback to the previous deployment; DB migrations are expand/contract and backward
  compatible for N-1.
- **Gateway/worker:** canary 10% → 50% → 100%; rollback on error rate > 1% or stop-latency regression.
- **Protocol:** N-1 support for 30 days; clients show "refresh to update" when below minimum.
- **Agent safety:** workspace/file kill switch; global disable via flag; runs paused, not lost.
- **Data:** snapshots + PITR; version restore; trash for 30 days.

### 11.6 Communications

- Launch narrative: "The design tool agents can actually use — and you can edit."
- Demo video: 90 seconds — manual design, prompt login, watch cursor, stop, edit, export code.
- Honest limits page: unsupported formats (`.fig` import), mobile viewer only, known gaps in export fidelity.
- Incident comms: status page + in-app banner + email for > 30 min outages.

---

## 12. Launch Metrics & Acceptance

### 12.1 Release gate dashboard

| Metric | GA threshold | Source |
|---|---|---|
| Activation (frame + run in first session) | ≥ 55% | PostHog |
| Agent structural success (eval) | ≥ 85% | Eval harness |
| Agent revert rate | ≤ 12% | Run events |
| Token coverage in agent-created nodes | ≥ 90% | Lint/eval |
| Manual edit rate after agent run | ≥ 60% of runs | PostHog |
| Crash-free sessions | ≥ 99.5% | Sentry/RUM |
| p95 pan/zoom frame | ≤ 16.7ms | RUM + bench |
| p95 stop latency | ≤ 250ms | Gateway |
| Code export compile rate (fixtures) | 100% | CI |
| axe critical/serious | 0 | CI |
| Support first response | ≤ 8 business hours | Helpdesk |

### 12.2 Definition of launch-ready

- All P0 parity tasks pass; no open P0/P1 bugs.
- Golden agent suite and adversarial suite pass their thresholds on the release model configuration.
- Security review signed; pen-test criticals closed.
- Backup restore drill passed within the last 30 days.
- Pricing, ToS, privacy, DPA, sub-processors published.
- Runbooks and on-call rotation active.

---

## 13. Governance

| Artifact | Cadence | Owner |
|---|---|---|
| Spec updates / RFCs | Per feature; reviewed within 2 business days | PM + tech lead |
| ADRs (`docs/adr/NNN-*.md`) | Per irreversible decision | Tech lead |
| Milestone demo | Bi-weekly | PM |
| Metrics review | Weekly (activation, agent success, perf, errors) | PM + data |
| Risk review | Bi-weekly; update §10 | PM |
| Security review | Per milestone; external pre-GA | Security |
| Retro | Per milestone | Eng manager |
| Design critique (chrome/agent UX) | Weekly during M4–M6 | Product designer |

**Spec change protocol:** propose a diff to the relevant doc → impact statement (milestones, tests, API) →
review → merge; implementation follows the merged spec, not a Slack thread.

---

## 14. Post-Launch Roadmap

| Horizon | Themes | Key items |
|---|---|---|
| 0–3 months | Retention & trust | Agent quality tuning per segment, template gallery, multi-agent UI, team libraries GA, Figma import (SVG/JSON best-effort), mobile viewer |
| 3–6 months | Depth | Responsive variant generation, AI reviewer filing comments, motion assistant, Flutter/RN export GA, plugin API beta, library marketplace |
| 6–12 months | Platform | Code round-trip (code → design updates), analytics-driven design suggestions, enterprise SSO/SCIM, data residency, on-prem/self-host option (Yjs), marketplace monetization |

---

## 15. Requirement Traceability

| Brief requirement | Document | Release gate |
|---|---|---|
| §6.1 canvas | `03` §2–4 | M1, parity 1–3, 8, 11–12, 17–18, 21–23 |
| §6.2 shapes | `03` §4–5 | M1, parity 2 |
| §6.3 text | `03` §6, `02` §8 | M1, parity 5 |
| §6.4 autolayout | `03` §7, `02` §7 | M1, parity 4 |
| §6.5 components | `04` §2, `02` §11 | M2, parity 6–8 |
| §6.6 design system | `04` §3, `02` §9 | M2, parity 9 |
| §6.7 assets | `04` §4, `02` §10 | M2, parity 10 |
| §6.8–6.9 layers/pages | `03` §8–9 | M1, parity 11–12 |
| §6.10 prototyping | `04` §5, `02` §12 | M6, parity 13–14 |
| §6.11 comments/collab | `04` §6, `02` §14 | M3, parity 15–16 |
| §6.12 import/export | `08` | M6, parity 19–20 |
| §7 AI layer | `05` | M4–M5, parity 24–25 |
| §8 UX | `01` | Continuous |
| §9–10 data/command model | `02`, `07` | M0–M2 |
| §11 workflow example | `05` §9 | M4 demo |
| §12 realtime | `07` §5–6 | M3 |
| §13 accessibility | `01` §12, `05` §6.5 | M7 |
| §14 performance | `06` §11 | M7 |
| §15–16 stack/architecture | `06` | M0 |
| §17–18 scope | `00` §7, this doc §7 | All |
| §19–24 non-goals/acceptance | `00` §7.4, §8.3 | Launch |

---

## 16. Open Decisions

| # | Decision | Needed by | Owner | Default |
|---|---|---|---|---|
| D1 | Default built-in model tier and provider routing | M4 | AI + PM | Balanced Claude, fallback OpenAI |
| D2 | Whether watch mode ships in MVP or V1 | M5 | PM | V1 (keep protocol support) |
| D3 | Free plan agent-run quota | S2 | PM + finance | 50/month |
| D4 | Data region for beta (US vs EU) | M3 | Infra | US, EU at GA |
| D5 | Whether to support `.fig` import in 6 months | Post-launch | PM | Best-effort after GA |
| D6 | Plugin API timeline | 6 months | Tech lead | Beta after library GA |
| D7 | Mobile editing beyond viewer | 12 months | PM | No |
| D8 | Open-source any packages (e.g., tokens/export) | Post-launch | Founders | Evaluate |
