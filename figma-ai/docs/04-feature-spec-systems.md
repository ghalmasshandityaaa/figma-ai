# 04 — Feature Spec: Systems

> **Status:** Normative for feature behavior. **Entity names, fields, and invariants:** [`02-data-model.md`](./02-data-model.md) is authoritative; this document never redefines them.
> **Audience:** implementation team (design, frontend, backend, agent platform, QA).
> **Scope owner:** Systems surface — components, design system, assets, prototyping, human collaboration, file/import/export feature surface.
> **Phase labels:** `MVP` ships in the first release; `V1` is post-MVP (0–6 months); `V2` is later. A phase label does not weaken MVP acceptance criteria unless explicitly stated.

---

## 1. Scope & How to Read

### 1.1 In scope

| Area | Sections | Primary entities |
|---|---|---|
| Components, variants, instances, component properties | §2 | `ComponentDefinition`, `ComponentNode`, `ComponentSetNode`, `InstanceNode`, `InstanceOverride`, `Library` |
| Tokens, variables, collections, modes, themes, styles | §3 | `Variable`, `VariableCollection`, `VariableMode`, `Theme`, `Style`, `Library` |
| Asset library, upload/import, SVG, images, fonts | §4 | `Asset`, `ImagePaint`, `Style` |
| Prototype links, triggers, actions, transitions, player | §5 | `Flow`, `Interaction`, `Trigger`, `Action`, `Transition`, `OverlaySettings` |
| Human presence, comments, multiplayer, versions, sharing | §6 | `PresenceState`, `CommentThread`, `Comment`, `Version`, `HistoryEvent`, `AuditLogEntry` |
| Pages, file browser, templates, import/export UI, handoff entry points | §7 | `Page`, `FileMeta`, `Asset`, `Version` |
| Cross-cutting errors, performance, deferred work | §8–§10 | — |

### 1.2 Out of scope (owned elsewhere)

| Topic | Document |
|---|---|
| Canvas, selection, shapes, text editing, auto layout, layers, pages core behavior | `03-feature-spec-core.md` |
| Agent runtime, plans, cursors, confirmations, prompt bar | `05-ai-agent-layer.md` |
| Document engine, command envelopes, undo/redo, CRDT mechanics, sync protocol | `07-state-model.md` |
| Code generation, framework mapping, token-to-code emission | `08-export-and-codegen.md` |
| Rendering architecture, workers, storage, deployment, perf internals | `06-architecture.md` |
| Auth, billing, workspace administration, SSO | `09-implementation-plan.md` / platform specs |

### 1.3 Reading conventions

- **MUST / MUST NOT** are testable requirements. **SHOULD** is a default that may be overridden by an explicit product setting. **MAY** is optional.
- **Command names** in mutation tables are `DesignCommand` kinds. A command is atomic and serializable; multiple commands are transported in a `CommandEnvelope` (02 §2). Names not defined in 02 are introduced here and listed in §11.
- **Operation names** (for example `export.run`) are not document mutations. They are jobs or workspace actions and MUST NOT appear in `HistoryEvent.inverse`.
- **UI-only state** (hover, preview theme, player runtime state, panel selection) is never written into `KanvasFile` unless the text names a persisted field.
- **Actor notation:** `u_*` human, `ag_*` agent, `SYSTEM` for engine-generated changes. Every mutation carries an actor and produces exactly one `HistoryEvent` (02 I10).
- **Time units:** milliseconds unless stated. **Geometry:** pixels. **Angles:** radians in the model; degrees in UI.
- **Selection in tables:** `N` = number of selected nodes; `I` = number of affected instances; `T` = token count.
- **Every destructive action** shows a confirmation naming the affected count and is undoable for 30 days through trash (`deletedAt`, `deletedBy`) or `Version` history.

### 1.4 Entity-to-surface map

| Entity | Created by | Edited in | Deleted by | Cross-references |
|---|---|---|---|---|
| `ComponentDefinition` | `component.create`, `componentSet.create` | Component panel, library publisher | `component.delete` | `nodeId`, `libraryId`, `semver` |
| `ComponentSetNode` | `componentSet.create` | Variants panel, canvas | `component.delete` | `variantAxes`, `children[]` |
| `InstanceNode` | `component.instantiate` | Canvas, properties panel | `node.delete` (core) | `componentId`, `componentProperties`, `overrides` |
| `InstanceOverride` | `instance.setOverride`, `instance.setComponentProperty` | Properties panel, agent tools | `instance.resetOverrides` | `nodePath`, `when`, `authoredBy` |
| `Variable` | `variable.create`, `variable.extract` | Tokens tab | `variable.delete` | `valuesByMode`, `scopes`, `category` |
| `VariableCollection` | `collection.create` | Tokens tab | `collection.delete` | `modes`, `variableIds` |
| `Theme` | `theme.create` | Themes tab | `theme.delete` | `modeByCollection`, `isDefault` |
| `Style` | `style.createStyle`, import | Styles tab | `style.deleteStyle` | `kind`, `definition`, `boundVariables` |
| `Asset` | `asset.upload`, `asset.register` | Assets tab | `asset.delete` | `hash`, `usage`, `license` |
| `Flow` | `prototype.setFlow` | Prototype mode | `prototype.deleteFlow` | `startNodeId`, `pageId` |
| `Interaction` | `prototype.link` | Prototype panel | `prototype.removeLink` | `trigger`, `actions`, `order` |
| `CommentThread` | `comment.create` | Comments panel, canvas pin | soft delete only | `anchorNodeId`, `position` |
| `Version` | `history.createVersion`, publish, agent pre-run | History panel | never (retention policy) | `revision`, `snapshotRef` |
| `Library` | `library.publish`, `library.link` | Libraries tab | `library.unlink` | `items`, `updatePolicy` |

### 1.5 Global interaction rules

| Rule | Behavior |
|---|---|
| Panel model | Right rail tabs: **Design**, **Prototype**, **Inspect**, **Comments**. Systems surfaces live under the left **Assets/Design System** panel and the **Design** tab. |
| Command palette | `⌘K` searches commands, assets, components, tokens, pages, and people. Results are grouped by namespace, max 8 per group, keyboard-first. |
| Selection context | Multi-select shows shared properties only; mixed values render as `Mixed`; editing a mixed property applies to all selected nodes in one `HistoryEvent`. |
| Async feedback | Operations >300ms show inline progress with cancel; >10s show estimated remaining time. Toasts auto-dismiss at 6s; errors persist until dismissed or resolved. |
| Warning badge | Amber dot on the owning tab; count in tooltip; click opens the validation list filtered to that surface. |
| Undo grouping | One gesture, one `HistoryEvent`; live drags coalesce via `HistoryEvent.coalesceKey` (02 §14). Bulk operations chunk at 500 nodes per envelope but share one `batchId`. |
| Keyboard | Every action in this document has a command-palette entry and a focusable control. Shortcuts are additive to the core map in `03-feature-spec-core.md`. |
| Limits | 50,000 nodes/file, 2,000 nodes/frame, 40 values per variant axis (02 §18.1). Product-level caps in §2.7, §3.3, §5.7 are stricter and return typed errors. |

### 1.6 Agent affordance legend

Each section ends with an **Agent-relevant affordances** table using three levels:

| Level | Meaning |
|---|---|
| `SAFE` | Agent may execute without confirmation when it holds the matching capability/scope; still attributable and undoable. |
| `CONFIRM` | Agent MUST request confirmation (`AgentAction.confirmation`) and show the affected objects. Denial is recorded in `AuditLogEntry`. |
| `PROTECTED` | Agent MUST NOT execute. Humans with owner role must unlock or perform the action. Attempts produce `GUARDRAIL_BLOCK`. |

`GuardrailPolicy.protectedNodeIds` defaults include every `COMPONENT` master, every published `COMPONENT_SET`, and every token collection marked `isLibrary`. `AgentCapability` and `PermissionScope` values come from 02 §15; this document does not invent capabilities.

---

## 2. Components & Variants

### 2.1 Data mapping (non-negotiable)

| Concept | Persisted as | Notes |
|---|---|---|
| Master | `ComponentNode` with `definitionId` | Lives in place on a `Page`; never a separate hidden page in MVP. |
| Published component | `ComponentDefinition` with `key`, `libraryId`, `semver`, `publishedVersion` | `nodeId` points to the `COMPONENT` or `COMPONENT_SET`. |
| Variant set | `ComponentSetNode` with `variantAxes: VariantAxis[]` | Children MUST be `COMPONENT` nodes; all axes identical (02 I5). |
| Variant | `ComponentNode` with `variantProperties` + its own `ComponentDefinition` carrying `componentSetId` and `variantProperties` | The set-level definition is the browser/publish unit; variant definitions are addressable for merge and rollback. |
| Instance | `InstanceNode` with `componentId`, `componentProperties`, `overrides[]` | Mirrored children are derived; overrides are sparse and path-based (02 §11.4). |
| Library | `Library` with `items.componentKeys`, `items.styleIds`, `items.collectionIds` | Assets are linked per-asset via `Asset.remoteLibraryId`; `Library.items` has no asset list in schema v1. |

**Key vs name.** `ComponentPropertyDefinition.key` is stable and never reused. Persisted `InstanceNode.componentProperties` is keyed by property **name**, matching the 02 §17.1 example. `InstanceOverride.when.propertyKey` uses the stable **key**. Renaming a property rewrites instance maps atomically but leaves all `when` conditions valid.

### 2.2 Creating components

| Entry point | Trigger | Result | Command |
|---|---|---|---|
| Create from selection | `⌘⌥K`, toolbar diamond, context menu | Each top-level selected node becomes a `COMPONENT` in place; children, transform, auto layout, styles, and `boundVariables` are preserved | `component.create` |
| Create from frame | Same, single `FRAME` selected | The frame's `type` becomes `COMPONENT`; `clipsContent`, background, and layout are preserved. No duplicate is created. | `component.create` |
| Multi-select | Same, 2+ top-level nodes | Default: N independent masters, named from each node. Confirmation lists names. | `component.create` (one envelope, N commands) |
| Create variant set | Context menu **Create component set**, or multi-select dialog | Selected `COMPONENT` nodes become children of one `COMPONENT_SET`; axes inferred and confirmed (see §2.6) | `componentSet.create` |
| Duplicate master | `⌘D` on a component, library card **Duplicate** | New `ComponentNode` + new `ComponentDefinition`; `key` is new; instances are not copied | `component.duplicate` |

Rules:

1. **Illegal targets.** A selection containing an `INSTANCE` is not converted silently. The dialog reports "N instances selected — detach first" and offers **Detach and create**. A selection containing an existing `COMPONENT` is skipped with a notice.
2. **Placement.** Masters stay where created. The layer panel shows the master badge; the component browser indexes all pages. Moving a master across pages is a core `node.reparent`; definitions follow `nodeId`.
3. **Name default.** The node's current name, trimmed. If a component with the same name exists, append ` 2`, ` 3`. Slash naming is preserved (`Button/Primary`).
4. **Atomicity.** Creating N masters is one `HistoryEvent`; undo restores every original node type and selection.
5. **Protected content.** Creating a component from a locked node is blocked; from a hidden node it is allowed and the master stays hidden only if the source was hidden (warning). Masters SHOULD be visible; a hidden master gets a warning badge.
6. **Bound content.** `boundVariables` and `styleRefs` survive conversion unchanged. A component master MAY contain unbound values; publishing surfaces a token-lint warning, not a block.

#### Naming conventions

| Pattern | Example | Meaning |
|---|---|---|
| `Group/Name` | `Button/Primary` | Slash creates browser hierarchy. Max 5 segments; each segment 1–64 chars; total ≤ 255. |
| `Group/Name/State` | `Button/Primary/Hover` | A standalone state master (not a variant). Discouraged when a variant set exists. |
| `Property=Value, Property2=Value2` | `state=hover, size=md` | Variant `COMPONENT` child name. Order MUST equal `variantAxes` order. |
| Component token path | `button/primary/bg/default` | Token namespace follows the component path; see §3.2. |

Reserved characters: `/` is hierarchy; `,` and `=` are reserved in variant names; leading/trailing whitespace is trimmed; repeated spaces collapse. Names may be duplicated, but duplicate sibling names inside a master trigger `override/ambiguous-path` (§2.7).

#### Master badge & component browser

- **Badge.** Four-diamond outline in the layer row, on-canvas selection chrome, and properties header. Chip text: `Master`, `Published v1.4.0`, or `Remote`. Protected masters add a lock glyph; dangling library masters add an amber `DANGLING` chip.
- **Browser.** Left panel **Assets → Components**. Sections: **Local**, then one per linked library. Each card shows name, thumbnail (last edited state), variant count, property count, published version, and used-by count (computed from `InstanceNode.componentId`). Grid/list toggle; search covers name, description, variant values, and library name; filters: library, page, has variants, modified since, used by selection.
- **Insertion.** Drag card to canvas, double-click, or `Enter` on focused card. Drag preview uses master bounds clamped to 320×320; instances are created at master size, not preview size.
- **Empty state.** "No components in this file" with **Create from selection** (disabled until a node is selected) and **Browse libraries**.

### 2.3 Instantiating, swapping, detaching

| Entry point | Behavior | Command |
|---|---|---|
| Drag from browser | Creates an instance centered at drop point. Dropping inside auto layout inserts at the computed index; default sizing is `HUG` unless `Shift` (FILL on primary axis). Dropping inside an `INSTANCE` body is rejected with "Detach to edit inside". | `component.instantiate` |
| Alt-drag from browser | Creates a **detached** copy at drop point (one gesture to customize), with `detachedSnapshot` populated; toast says "Detached copy". | `component.instantiate` + `component.detach` |
| Alt-drag selected instance on canvas | Duplicates the instance, preserving `componentProperties` and `overrides`. | `component.instantiate` (copy) |
| Copy/paste | Preserves overrides in-file. Across files, resolves master by `componentKey`; if absent, creates a dangling instance with inline `detachedSnapshot` and a warning. | `component.instantiate` |
| Component picker | `⌘⇧K`; search; `Enter` inserts at selection center or replaces a selected instance (swap mode); `Esc` closes. | `component.instantiate` or `component.swap` |
| Swap via right panel | **Swap component** picker on a selected instance. Shows only compatible candidates first (same variant axes where possible), then all components. | `component.swap` |
| Detach | Converts instance + mirrored subtree to ordinary nodes; overrides become concrete values; `detached: true`; `componentId` retained as provenance; `detachedSnapshot` retained. | `component.detach` |
| Restore link | Re-resolves `componentId` and clears `detached`; warns that detach-time edits will be lost; requires confirmation. | `instance.relink` |
| Reset overrides | All, by property, or by subtree. Confirmation shows the count. | `instance.resetOverrides` / `instance.resetOverrides` |
| Go to master | Selects and frames the master; for remote libraries opens a read-only preview or library card if access is denied. | navigation only |

**Swap semantics.** `component.swap` changes the root `componentId`. Nested swaps use an `InstanceOverride` with `property: 'componentId'` and a `nodePath`. During a root swap:

1. New master structure replaces the mirrored tree.
2. Overrides are rebased by `nodePath` + property. Matching paths and compatible types survive.
3. Unresolvable overrides move to `orphanedOverrides` (02 §11.4). They are never auto-deleted.
4. `componentProperties` are matched by property name, then by `key`; removed properties are dropped with a warning count.
5. The instance's own transform is preserved; if the new master is larger than the parent, overflow warning appears.

**Detach vs dangling.** `detached: true` + resolvable `componentId` = **DETACHED**. `detached: true` + missing master/library = **DANGLING** (02 I6). Both retain `detachedSnapshot`; only DANGLING shows **Restore link** in the header. Detaching is undoable; relinking is destructive to detach-time edits and requires confirmation.

### 2.4 Instance overrides UX

**Overridable properties.** The matrix is normative for MVP. "Root" means the `INSTANCE` node itself; "mirrored" means a node inside its derived subtree.

| Property path | Root | Mirrored | Notes |
|---|---|---|---|
| `characters` | — | Yes (`TEXT`) | Never writes the master (02 §8). |
| `textStyle.fontSize`, `fontStyle`, `lineHeight`, `letterSpacing`, `textAlignHorizontal`, `textCase` | — | Yes | Text color is `fills.*.color`. |
| `fills`, `strokes`, `effects`, `opacity`, `visible`, `cornerRadius` | Yes | Yes | Full array replacement; per-paint override is not exposed in MVP. |
| `boundVariables.<path>` | Yes | Yes | Value is a `VariableAlias`; binding stays theme-reactive. |
| `componentProperties.*` | Yes | Nested instance | See §2.5. |
| `componentId` | Root swap | Nested instance swap | Root swap is `component.swap`, not an override. |
| `transform` (position/size) | Yes (node transform) | No | Child geometry is solved by auto layout; child transform overrides are deferred. |
| `autoLayout.*` | Yes, root only | No | Changing root gap/padding is stored as an override at path `[]`. |
| `children[]` structure | — | No | Use slots (§2.7). |
| `type`, `id`, `permissions`, `locked` | — | No | Structural and permission fields are never overridable. |

**Override indicators.**

| Surface | Indicator |
|---|---|
| Layer row | 4px filled dot in actor-neutral accent; hover tooltip lists override count. |
| Properties row | Row label turns accent; reset arrow appears on hover and is keyboard reachable. |
| Instance header | `3 overrides` chip; click opens a list grouped by node path; each row has **Reset**. |
| Canvas | Selecting a modified child shows a dashed 1px outline inside the instance selection. |
| Layers filter | **Modified** toggle shows only nodes with overrides. |
| Multi-instance selection | `Mixed` plus **Reset all**; resetting applies to every selected instance. |

**Resolution and precedence.** Overrides apply in this order: base overrides (no `when`) in authored order, then conditional overrides whose `when` matches the current component property, in `InstanceOverride` array order. Last matching write wins. Reset removes the specific `(nodePath, property, when?)` tuple. Conditional overrides that no longer match are retained, not deleted.

**Exposed nested properties.** A parent component MAY expose a property that targets a nested instance. The property name uses a path: `{childLayerName}/{propertyName}` (for example `leadingIcon/size`). The definition lives on the parent's `ComponentDefinition.propertyDefinitions`; `key` is parent-scoped. The property control on the parent instance writes an override whose `nodePath` ends at the nested target. Exposure is available for `BOOLEAN`, `TEXT`, `INSTANCE_SWAP`, and `VARIANT`; `NUMBER` exposure is deferred until property-to-layer mapping is schema-backed. If the nested target is removed, the property becomes **broken**: control disabled, tooltip names the missing path, and deletion is offered.

### 2.5 Component properties

| Type | Control | Value | Target resolution | Default | Limits |
|---|---|---|---|---|---|
| `BOOLEAN` | Switch | `boolean` | Mirrored node named exactly like the property; toggles `visible` | Required | 1 property → 1 layer |
| `TEXT` | Text field | `string` | `TEXT` node named like the property; writes `characters` | Required, may be `""` | 10,000 chars |
| `INSTANCE_SWAP` | Component picker | `{ componentKey: string }` | `INSTANCE` node named like the property; changes its `componentId` | Required; first preferred value if any | 20 preferred values |
| `VARIANT` | Select | `string` | Must equal a `variantAxes` name; edits the axis value | First axis value | Mirrors axis values; 40 max |
| `NUMBER` | Stepper + drag | `number` | Numeric property path on the named node: `itemSpacing`, `cornerRadius`, `opacity`, `fontSize`, `width`, `height` | Required | Finite; UI clamps per target (spacing 0–1000, opacity 0–1, size 1–10000) |

**Creation flow.** Select a master → **Properties** section → `+` → type → name → target layer (pre-filtered by compatible type) → default → optional description. Creating a property is one `componentProperty.create` command; the definition array order is the control order.

**Editing rules.**

1. **Names** are unique per component set, case-insensitive, 1–64 chars; `/`, `,`, `=` are rejected. A non-VARIANT name MUST NOT collide with an axis name.
2. **Rename** keeps `key`; rewrites `InstanceNode.componentProperties` keys on all instances in one atomic command. `InstanceOverride.when.propertyKey` is unaffected.
3. **Type change** is blocked in MVP. The dialog offers **Delete and recreate**; affected overrides are listed and moved to orphan cleanup after recreation.
4. **Default change** updates instances that have no explicit value. Instances with an explicit value show a **set** dot and do not change.
5. **Delete** requires confirmation with instance count; dependent `componentProperties` values and `when` overrides are removed in the same command; undo restores them.
6. **Preferred values** are ordered; drag to reorder; duplicates rejected. A preferred value whose component is missing remains in the list with a **Missing** badge and is skipped at pick time.
7. **Conflicts.** If a property name matches multiple mirrored layers, the first in `children[]` order wins and `component/property-conflict` is raised. If a VARIANT property and an axis diverge, the command is rejected; `ComponentSetNode.variantAxes` is authoritative and the property definition is a generated mirror.

**Property controls panel.** Controls render in definition order above the layers list, with a **Mixed** state for multi-selection. Each control has: label, info tooltip from `description`, value, reset-to-default, and an **Expose** action for nested targets. Changing a control writes one override or one `componentProperties` entry; changes commit on blur or `Enter`, never per keystroke, except text fields which debounce at 300ms.

### 2.6 Variants

**Creating a variant set.** Select 2+ `COMPONENT` nodes → **Create component set**. The dialog previews inferred axes:

- If names follow `Name/Value` and all selected nodes share the prefix, the differing suffix becomes axis `state`.
- Otherwise an axis `variant` is proposed with values `v1..vN`.
- The user MUST confirm. `componentSet.create` reparents components under a new `COMPONENT_SET`, sets `variantAxes`, and assigns `variantProperties` to each child.

**Rectangular matrix.** For axes `A × B × C`, the set MUST contain exactly `|A|·|B|·|C|` `COMPONENT` children. Adding a value to any axis auto-creates every missing combination as an empty master (02 §11.3). The confirmation states the exact new combination count before writing. Empty masters render as a 100×100 dashed frame with the variant name, are selectable, and carry `variant/empty-master`. Publishing is blocked while any empty master is in the publish set.

| Operation | Behavior | Command |
|---|---|---|
| Add variant value | Adds value to axis; creates all combinations; max 40 values/axis and 2,000 combinations | `variant.add` |
| Remove variant value | Deletes the entire row/column; confirmation names affected instances; instances of removed variants become dangling with snapshot | `variant.remove` |
| Rename axis | Rewrites `variantAxes.name`, child names, and `variantProperties` keys; warns that `when` conditions keyed by the old variable property may orphan | `variant.rename` |
| Set default | Moves the chosen variant to first position in `children[]`; default is defined as the first `COMPONENT` child (no extra field exists) | `variant.setDefault` |

**Axis naming.** Axis names and values use the same character rules as property names; comparison is case-insensitive, display preserves case. Axis order is `variantAxes` order and defines the order in the properties panel and in `Property=Value` names. Deleting the last value of an axis is rejected; deleting an axis deletes all its values after confirmation.

**Default variant.** Deterministic: first `COMPONENT` child in `children[]`. The UI badges it **Default**. Unknown combinations fall back to this child and raise `instance/variant-mismatch` (02 §11.3). Changing default is a publish-visible change and is classified as **major** if published instances relied on the old default.

**State preview.** Hovering a variant card in the browser or the instance's variant dropdown shows a read-only thumbnail. **Preview states** cycles through values whose names match `default|hover|pressed|focus|disabled|loading|error` at 600ms per state inside the properties panel only; it never mutates the instance. Clicking a variant commits a swap or `componentProperties` change.

**Prototype interaction.** `Action.CHANGE_VARIANT` sets `componentProperties[propertyKey]` for a VARIANT property on the target instance at runtime. It is ephemeral (§5.12). Variant changes swap the mirrored subtree and re-run hit testing immediately; timers on newly visible nodes arm from the change time, timers on unchanged nodes are not restarted. Variant-to-variant animated transitions are deferred; use `NAVIGATE` + `SMART_ANIMATE` between frames for animation.

### 2.7 Nesting, slots, and override paths

**Override path resolution.** `InstanceOverride.nodePath` is resolved segment by segment against child `name` values, starting at the instance root's children. A segment MAY address a nested instance's mirrored child. Paths are capped at 12 segments; deeper paths violate `override/path-depth`. If two siblings share a name, the first in `children[]` order is used and `override/ambiguous-path` is raised. Renaming a master child does not rewrite paths; affected overrides become orphaned and appear in **Cleanup → Orphaned overrides** with the old path and last value.

**Slots.** MVP slot pattern uses `INSTANCE_SWAP`, fully representable in 02:

1. In the master, create a `FRAME` named `Slot` (or `Slot/{name}`) with auto layout and `clipsContent: true`.
2. Place a placeholder `INSTANCE` inside it, named `SlotContent`, from a local `Placeholder/Empty` component.
3. Add an `INSTANCE_SWAP` property named `slot` (or `slot/{name}`) targeting `SlotContent`, with preferred values.
4. Consumers swap the slot through the property control. The replacement must fit the slot frame; overflow is clipped, with an overflow warning badge.
5. Nested slots compose to depth 12; each level adds one override path segment.

Arbitrary child insertion into an instance (true children-as-slot with instance-local structure) is **deferred**: 02 mirrors instance children and stores content only through overrides. Workaround is detach, which loses the link. This is a known schema gap, not a UI omission.

**Performance limits.** Product caps: 1,000 mirrored nodes per master (warning at 500), 12 nesting levels, 100 properties per set, 2,000 variant combinations per set. Instance rendering must not resolve overrides on every frame; resolution is cached per `(instanceId, revision, componentRevision, themeId)`.

### 2.8 Libraries

| Kind | Definition | Phase | Publish target |
|---|---|---|---|
| `INTERNAL` | Components/styles/collections from the current file, published for reuse within the same workspace | MVP | Workspace |
| `TEAM` | Curated workspace library with semver, update policy, changelog | V1 | Workspace/team |
| `EXTERNAL` | Linked library from another workspace or an imported artifact | V2 | Cross-workspace |

**Publish flow.**

1. **Select items.** Components (sets and standalone), styles, collections. Hidden items (`hiddenFromPublishing`) are excluded and counted.
2. **Validate.** Protected masters MUST be unlocked (02 §11.4). Empty variants block. Broken aliases, missing fonts, and unlicensed assets are warnings; the publisher shows them with **Publish anyway** only for warnings.
3. **Diff.** Compute against the last immutable artifact: added / changed / removed / breaking, grouped by item, with per-component property and variant diffs.
4. **Version.** Auto-suggest semver; publisher MAY raise it, and MAY lower it only after typing the current suggestion and acknowledging consumer impact. Changelog required for major, optional otherwise.
5. **Confirm.** Typed confirmation for major. Publish writes `Library` metadata (`version`, `publishedAt`, `changelog`), updates each `ComponentDefinition.publishedVersion`/`semver`, and stores an immutable `LibraryPublish` snapshot for rollback.

**Semver rules.**

| Bump | Triggers | Examples |
|---|---|---|
| Patch | Visual/value change with no API change | color value, spacing, icon path, copy, description |
| Minor | Additive, backward-compatible | new component, new variant value, new optional property, new token, new mode, new style |
| Major | Consumer-visible break | remove/rename component or property, change property type, remove variant value, change default variant, remove token/mode, change token resolved type/category, rename token |

**Diff summary.** Header counts: `+12 added · 4 changed · 1 removed · 2 breaking`. Rows expand to property-level and variant-level changes. Breaking rows are pinned first and cannot be collapsed without expanding once.

**Consuming updates.**

| Surface | Badge | Action |
|---|---|---|
| Library card | `Update v1.4.0 → v1.5.0` | Review diff |
| Component browser card | dot (patch), count badge (minor), amber warning (major) | Review update |
| Instance header | `Update available` when master accepted version < published | Accept / Ignore |
| File banner | One summary for all libraries | Review all |

**Accept update (three-way merge).** Base = published snapshot at the instance's accepted version. Local = instance overrides and `componentProperties`. New = new master.

1. Replace mirrored structure with new master.
2. Rebase overrides by path/property; compatible paths apply.
3. Missing paths become `orphanedOverrides`; never auto-delete.
4. Property values survive by name, then key; removed types reset with a warning.
5. Overrides whose value equals the old default are treated as non-customizations and dropped in favor of the new default.
6. Accept is atomic per component, one `HistoryEvent` with `origin: 'LIBRARY_UPDATE'` and inverse commands.

**Ignore.** Hides the badge for that component until the next publish. Ignore is library-relationship UI state outside `KanvasFile`; it never mutates the document. `Library.updatePolicy` `AUTO_ACCEPT_PATCH` applies patches only and never minors/majors; `NOTIFY` badges all; `MANUAL` requires review for all.

**Breaking changes.** A major publish lists each break with consumer count and required action:

| Break | Consumer impact | Required action |
|---|---|---|
| Property removed/renamed | `componentProperties` key or `when` target missing | Remap by key or reset to default |
| Property type changed | Value incompatible | Reset, with warning |
| Variant value removed | Instances on that combo dangle | Swap to another variant or accept snapshot |
| Component removed | Instances dangle | Relink or detach |
| Token removed | Bindings/aliases broken | Replace with another token or detach |
| Mode removed | Theme references invalid | Remap theme modes |

**Version history & rollback.** Per component key: append-only list of `{version, publishedAt, publishedBy, diffSummary, snapshotRef}`. **Preview** opens a read-only canvas overlay with a version banner. **Rollback** publishes the old content as a **new** version (history is never rewritten); semver is suggested from the rollback diff and major requires confirmation. Rollback affects consumers through the normal update flow, never by mutating their overrides.

**Unlink/deleted library.** `library.unlink` removes consumption links; instances become DANGLING with `detachedSnapshot`, keep rendering, and show **Restore link** if a library with matching keys is re-linked. Deleted library cards remain in the Libraries tab as **Deleted** with last version and link count until the owner removes them.

### 2.9 Component guardrails

| Action | Human editor | Agent | Notes |
|---|---|---|---|
| Edit master structure | Allowed unless protected | `PROTECTED` | 02 §11.4: agents cannot edit masters; they write overrides. |
| Create local component | Allowed | `SAFE` with `COMPONENT_MANAGE` | New master is unprotected by default. |
| Instantiate / set properties / override | Allowed | `SAFE` with `DESIGN_EDIT` | Instance-only writes. |
| Detach instance | Allowed | `CONFIRM` | Loses link; protected subtree raises risk. |
| Delete master | Owner/editor with confirm | `CONFIRM` + `DESTRUCTIVE` | Instances dangle; never silent. |
| Publish library | `publish` scope | `CONFIRM` | Requires `publish` scope; protected master blocks. |
| Unlock protected master | Owner only | `PROTECTED` | `component.unprotect`; audited. |
| Change library policy | Owner | `PROTECTED` | `library.setPolicy`. |
| Edit published property definitions | Allowed with major warning | `CONFIRM` | Key stability enforced. |

Agents touching a master under `GuardrailPolicy.requireAnnotationOnMasters` MUST add an `AgentAnnotation` of kind `HANDOFF` or `RATIONALE`; the command batch is rejected otherwise.

### 2.10 Component empty & error states

| State | UI | Recovery |
|---|---|---|
| No components | Empty browser with create/browse actions | Create from selection |
| No search results | "No components match" + clear filters | Clear |
| Dangling master | Amber `DANGLING` chip; instance renders snapshot | Restore link, relink library, or detach |
| Deleted library | Library card `Deleted`; link count | Re-link matching library or detach all |
| Missing font in master | Text renders fallback; instance warning | Font resolution flow (§4.8); publish warning |
| Variant mismatch | Instance swaps to default + warning badge | Choose valid combination |
| Ambiguous override path | Override kept; cleanup panel entry | Rename duplicate layers or reset |
| Property conflict | Control disabled + tooltip | Rename property/layer or delete |
| Protected master | Lock chrome; edit attempts denied | Owner unlocks |
| Empty variant master | Dashed preview + `Empty` badge | Fill or remove; publish blocked |
| Swap incompatible | Warning listing dropped overrides before commit | Confirm or cancel |
| Update merge conflict | Per-component conflict list | Resolve field-by-field or ignore |

### 2.11 Components acceptance criteria

1. Converting a frame to a component preserves `transform`, `children[]`, `autoLayout`, `fills`, `styleRefs`, and `boundVariables`; undo restores `type: 'FRAME'` and the exact selection.
2. Creating N components from N selected nodes produces N `ComponentDefinition` records with unique names and one `HistoryEvent`.
3. Dragging a component to canvas creates one `InstanceNode` whose `componentId` resolves and whose bounds equal the master's at creation.
4. Editing instance text changes only the instance; the master `characters` is byte-identical after the operation; the override count increments by exactly 1.
5. Resetting that override restores the master text in ≤50ms for a 200-node instance and is undoable.
6. Renaming a component property updates every instance's `componentProperties` key in one command; `when.propertyKey` conditions still evaluate.
7. Adding a value to a 2×2 variant set produces exactly 6 variants, 2 new empty masters, and a publish block until both are filled.
8. Swapping an instance to a component with 3 matching and 1 missing override path preserves 3, orphans 1, and never deletes the orphan.
9. Publishing creates an immutable version and a diff with correct counts; accepting an update applies a three-way merge and produces `origin: 'LIBRARY_UPDATE'`.
10. An agent command that edits a protected master is rejected with `GUARDRAIL_BLOCK` and an `AuditLogEntry`; the document revision does not change.
11. A 41st value on one axis and a 2,001st combination are rejected with typed limit errors and actionable messages.
12. Instantiating a 200-node master has p95 ≤50ms; variant swap on the same master p95 ≤100ms.

### 2.12 Agent-relevant affordances — components

| Capability | Level | Constraint |
|---|---|---|
| Instantiate component, set text/boolean/swap/variant values | `SAFE` | Must use library components when a matching component exists (P7 token/component-first). |
| Override visual properties on an instance | `SAFE` | Writes `InstanceOverride`, never master edits. |
| Create component from agent-created nodes | `SAFE` | `requireAnnotationOnMasters` applies if enabled. |
| Detach, reset all overrides, delete instance | `CONFIRM` | Count and affected nodes shown. |
| Create/remove variant values, edit property definitions | `CONFIRM` | Publish impact preview required. |
| Edit master internals, unlock, publish, delete master | `PROTECTED` | Human owner only; `publish` scope still requires confirmation. |

---

## 3. Design System Layer

### 3.1 Design System panel information architecture

The panel is a single left-rail surface at **Assets → Design System**, opened with `⌥2`; it is resizable (280–480px) and remembers width per user.

| Tab | Contents | Primary actions | Counters |
|---|---|---|---|
| **Tokens** | Collections tree, mode matrix, token table, lint summary | Create token/collection, bind, import/export, publish | Tokens, aliases, unused, lint errors |
| **Styles** | FILL / STROKE / EFFECT / TEXT / GRID groups | Create, edit, apply, detach, publish | Styles, unused, remote |
| **Components** | Component browser (§2.2) | Create, instantiate, publish | Components, variants, updates |
| **Themes** | Theme cards with mode selections, default marker, preview | Create, duplicate, set default, preview | Themes, missing modes |
| **Libraries** | Linked libraries, publish history, updates | Publish, link, review update, rollback | Libraries, pending updates |

Header controls: scope switcher (**This file** / **{Library name}**), search (`/` focuses), filter, and **Publish changes** with a count. The footer shows active theme and a **Preview theme** selector. Opening a token from search or a binding chip deep-links to its row and expands its collection.

### 3.2 Token taxonomy, naming, aliasing, lint

A **token** is the user-facing view of a `Variable` (scalar/color) or a `Style` (composite). `Variable.category` is authoritative when set; otherwise it is derived from the name and MUST match the derived value (`token/category-mismatch` otherwise).

| Category | Backing entity | Resolved type / kind | Naming pattern | Example |
|---|---|---|---|---|
| Color primitive | `Variable` | `COLOR` | `color/palette/{hue}/{step}` | `color/palette/blue/500` |
| Color semantic | `Variable` | `COLOR` | `color/{role}/{element}/{state}` | `color/bg/surface/raised`, `color/fg/muted` |
| Color component | `Variable` | `COLOR` | `{component}/{part}/{property}/{state}` | `button/primary/bg/hover` |
| Space | `Variable` | `FLOAT` | `space/{role}/{size}` | `space/gap/md`, `space/inset/card` |
| Radius | `Variable` | `FLOAT` | `radius/{role}/{size}` | `radius/control/md` |
| Border | `Variable` | `FLOAT` | `border/width/{size}` | `border/width/hairline` |
| Shadow | `Style` | `EFFECT` | `shadow/{level}` | `shadow/overlay` |
| Elevation | `Style` | `EFFECT` | `elevation/{level}` | `elevation/2` |
| Typography (composite) | `Style` | `TEXT` | `typography/{role}/{size}` | `typography/body/md` |
| Typography (scalar) | `Variable` | `STRING`/`FLOAT` | `typography/{property}/{name}` | `typography/family/sans`, `typography/size/body` |
| Motion | `Variable` | `FLOAT` or `STRING` | `motion/{kind}/{name}` | `motion/duration/fast`, `motion/easing/standard` |
| Opacity | `Variable` | `FLOAT` | `opacity/{role}` | `opacity/disabled` |
| Size | `Variable` | `FLOAT` | `size/{role}/{size}` | `size/icon/md`, `size/control/sm` |
| Grid | `Style` | `GRID` | `grid/{name}` | `grid/desktop/12` |

**Naming grammar.** Lowercase kebab-case per segment; `/` separates hierarchy; segment regex `^[a-z][a-z0-9]*(-[a-z0-9]+)*$`; 1–64 chars per segment; total ≤255; 2–5 segments recommended, 6 max. Numeric scales use `50, 100, 200, …`; semantic states use `default|hover|pressed|focus|disabled|loading|error`. Category prefix is mandatory for Variables and for Styles of kind FILL/STROKE/EFFECT/TEXT/GRID (`color/`, `shadow/`, `elevation/`, `typography/`, `grid/`).

**Classification (must match 02 §9.1).**

1. **Component token** when segment 1 matches a registered component namespace (first segment of any `ComponentDefinition.name`, case-insensitive) and the path has ≥4 segments.
2. Else **semantic** when the value resolves through a `VariableAlias` and the path has a role segment at position 2. Role lexicon: `bg, fg, text, surface, border, accent, brand, danger, success, warning, info, focus, overlay, inverse, link, selection, disabled`.
3. Else **primitive** when the value is literal and the path uses a primitive prefix (`color/palette/`, `space/scale/`, `radius/scale/`, `motion/duration/`, `opacity/scale/`, `size/scale/`).
4. Everything else is `unclassified` and raised as `token/ambiguous-role` (info). It is still usable but excluded from semantic export conventions.

| Alias edge | Allowed | Rule |
|---|---|---|
| Primitive → literal | Yes | Primitives MUST be literal. `token/primitive-alias` otherwise. |
| Semantic → primitive | **Yes** | This is the intended layering. 02 does **not** forbid it. |
| Semantic → semantic | Yes | Cycle check; depth limit 32 (02 §9.1). |
| Component → semantic | Yes, exactly one hop | May alias semantic only. |
| Component → component | No | `token/component-alias-chain`. |
| Component → primitive | **No** | `token/component-alias-primitive`, mandated by 02 §9.1. |
| Any → different type | No | `FLOAT`/`COLOR`/`STRING`/`BOOLEAN` must match exactly. |
| Style → variable | Yes | `Style.boundVariables`; scope/type checked. |

**Lint rules.**

| ID | Severity | Check | Fix |
|---|---|---|---|
| `token/name-format` | Error | Grammar above, category prefix, reserved chars | Rename |
| `token/category-mismatch` | Error | Explicit `category` differs from derived | Set category or rename |
| `token/primitive-alias` | Error | Primitive value is `VariableAlias` | Inline value or reclassify |
| `token/component-alias-primitive` | Error | Component token aliases primitive | Point to semantic |
| `token/component-alias-chain` | Error | Component aliases component | Flatten to semantic |
| `token/type-mismatch` | Error | Alias resolved types differ | Repoint |
| `token/scope-mismatch` | Error | Binding target outside `scopes` | Change scope or binding |
| `token/alias-cycle` | Error | Cycle in alias graph | Break cycle |
| `token/alias-depth` | Warning | Depth >4 (hard max 32) | Flatten chain |
| `token/semantic-literal` | Warning | Role-named token has literal value | Alias a primitive |
| `token/component-unscoped` | Warning | Component token used outside its namespace | Bind semantic instead |
| `token/orphan` | Info | No usage for 30 days and unpublished | Deprecate/delete |
| `token/deprecated-in-use` | Warning | Deprecated annotation with active usage | Migrate |
| `token/hardcoded-value` | Warning | Node value equals a token but is unbound | Bind |
| `token/contrast` | Warning | Text/background below 4.5:1 (3:1 for ≥24px or bold ≥18.66px) | Adjust token |

Lint runs after every token command (debounced 200ms), on library publish (blocking for errors), and in the agent run summary. Warnings never block edits; they block publish only when the publisher opts into strict mode.

### 3.3 Collections & modes

A `VariableCollection` groups variables that share modes. `modes` is ordered; `defaultModeId` MUST be present in `modes`. A `VariableMode` name is unique within its collection, 1–32 chars.

| Collection | Modes (ordered) | Purpose | Phase |
|---|---|---|---|
| `Theme` | `light`, `dark` | Surface/text color scheme | MVP |
| `Brand` | `brand-a`, `brand-b` | Multi-brand color/type overrides | V1 |
| `Density` | `compact`, `comfortable`, `spacious` | Spacing and control sizing | V1 |
| `Contrast` | `default`, `high` | Accessibility contrast mode | V2 |

**Creation.** **Tokens → + Collection** → name, modes (at least 1), default mode. New tokens require a writable collection; if none exists, the UI offers to create `Local` with one mode (`default`). A collection may be `isLibrary` and linked; `hiddenFromPublishing` excludes it from publish.

**Mode editing.**

- The mode matrix shows tokens as rows and modes as columns. Each cell holds a literal or an alias; changing one cell writes one `variable.setValue` for that mode only.
- **Copy to modes** copies the edited value to selected modes (one command per mode, one envelope). It never links modes; there is no mode-link field in 02.
- Adding a mode copies the default mode's values, then marks the new column **Unreviewed** until every token has an explicit value or is intentionally left equal to default.
- Renaming a mode is metadata-only; `Theme.modeByCollection` references the mode id, so themes are unaffected.
- Deleting a mode requires reassigning every theme that references it. The dialog lists affected themes and offers a replacement mode. Deleting the last mode is rejected.
- Deleting a collection requires deleting/migrating all variables; bound nodes are detached or rebound per §3.8. Blocked while any theme references a mode in it.

**Mode switching UX.** The top-bar theme switcher selects a `Theme`, not an individual mode. Individual mode changes happen only in the token matrix or via a theme. Switching a theme applies within 100ms p95 for 5,000 bound nodes; the canvas does not reload.

### 3.4 Themes

A `Theme` stores exactly one mode per collection in `modeByCollection` and MAY carry `overrides` (variable id → value) for one-off brand tweaks. Overrides are limited to 100 entries per theme and, in the editor, accept literals only; imported aliases are resolved once at load and normalized.

| Operation | Behavior | Command |
|---|---|---|
| Create | Wizard: name, one mode per collection, optional duplicate from existing theme | `theme.create` |
| Update modes | Change one or more collection modes; preview before commit | `theme.update` |
| Duplicate | Copies modes and overrides with a new name | `theme.duplicate` |
| Set default | Sets `FileMeta.defaultThemeId` and `Theme.isDefault`; exactly one default | `theme.setDefault` |
| Delete | Blocked if default; otherwise remove; on-canvas preview resets | `theme.delete` |

**Default theme.** `FileMeta.defaultThemeId` wins; if absent, the theme with `isDefault: true`; if neither, each collection's default mode. A file with no themes resolves every collection to `defaultModeId`.

**Preview as theme.** Selecting **Preview** sets a UI-only `previewThemeId` (never persisted, never in `HistoryEvent`), shows a 32px banner `Previewing Dark — Esc to exit`, and applies to canvas, token swatches, and prototype preview. It does **not** apply to exports or code generation unless the export dialog's theme option explicitly selects it. Closing or `Esc` restores the previous theme in one frame.

**Cascade into instances and prototypes.**

- Resolution is exactly 02 §9.1, in editor, player, and exporters.
- Instance overrides that store a `VariableAlias` re-resolve under the active theme. Overrides that store literals stay literal.
- Styles resolve their `boundVariables` under the active theme; node-level bindings win (§3.7).
- Prototype player starts with the theme passed in the share link, else `FileMeta.defaultThemeId`. `Action.SET_VARIABLE` with `modeId` changes the session mode for that collection and re-resolves bound nodes; it never writes a `Theme`.
- Themes are file-scoped and are **not published** (`Theme` has no `libraryId` in 02). Collections, variables, and styles are published; consumers create their own themes from the published modes. Publishing a theme selection is a V2 feature requiring a schema addition.
- Per-frame theme pins are not representable in schema v1 and are deferred; use separate frames or prototype `SET_VARIABLE`.

### 3.5 Binding UX

| Bindable target | Variable scope required | Notes |
|---|---|---|
| `fills.{i}.color` (solid) | `ALL_FILLS`, `FRAME_FILL`, `SHAPE_FILL`, or `TEXT_FILL` | Gradient stop paths are bindable (`fills.0.stops.1.color`); image paints are not. |
| `strokes.{i}.paint.color` | `STROKE_COLOR` | Solid strokes only in MVP. |
| `effects.{i}.color` / `radius` / `spread` | `EFFECT_COLOR` / `ALL_SCOPES` | Shadow color tokens supported. |
| `itemSpacing`, `counterSpacing`, `padding.*` | `GAP` | Negative spacing not allowed by layout validator. |
| `cornerRadius` | `CORNER_RADIUS` | Per-corner binding is deferred; use the uniform value. |
| `width`, `height`, `minSize`, `maxSize` | `WIDTH_HEIGHT` | Not bindable on `FILL`/`HUG` axes. |
| `opacity` | `OPACITY` | 0–1. |
| `textStyle.fontSize`, `lineHeight`, `letterSpacing`, `paragraphSpacing` | `FONT_SIZE` / `LINE_HEIGHT` / `LETTER_SPACING` / `PARAGRAPH_SPACING` | Text style edits write ranges or base style per selection. |
| `textStyle.fontFamily`, `fontStyle` | `FONT_FAMILY` / `FONT_STYLE` | Font must be registered (§4.8). |

**Binding flow.** Hovering a bindable row reveals the token icon. Clicking opens the token picker: search, **This file** / library tabs, collection groups, resolved swatch, alias chain, and mode values on hover. Selecting a token writes `boundVariables[path] = { type: 'VARIABLE_ALIAS', id }` through `variable.bind`. Type/scope validation is synchronous and rejects with a typed error before any mutation.

**Bound-state visuals.** The row shows a token chip (`color/bg/surface/raised`) with a link glyph and resolved swatch. The value field becomes read-only; clicking opens the picker. Tooltip shows the value in every mode of every collection containing the token, the alias chain, and the token's owning library. Multi-selection shows `Mixed` when bindings differ. A broken/cyclic binding renders magenta plus `Unresolved` and offers **Repair**.

**Detach / swap.**

| Action | Behavior | Command |
|---|---|---|
| Detach | Removes the alias and writes the currently resolved value as a literal | `variable.unbind` |
| Swap | Replaces the alias at the same path; validates type/scope | `variable.bind` |
| Reset to style | Removes node binding so the style binding applies again | `variable.unbind` |
| Create variable from value | Creates a token in a chosen collection/mode at the current value, then binds | `variable.extract` |
| Find usages | Lists nodes, styles, component definitions, themes, prototype actions, and code syntax references | read-only |
| Bulk rebind | Replaces token A with B across selection, page, or file | `variable.rebind` |

**Create from value** follows `tokenEditBehavior` (02 §9.1): `CREATE_VARIABLE` opens the create dialog, `DETACH` leaves a literal, `ASK` shows both. The dialog pre-fills category, scope, collection, and a name suggestion; it refuses duplicate names without a suffix and shows the resolved value per mode.

**Find usages** groups by surface with counts: nodes (click filters canvas), styles, component masters, themes (overrides), prototype `SET_VARIABLE`/`CHANGE_VARIANT` actions, and code syntax. A usage list of 0 still allows delete (with warning).

**Bulk rebind** is all-or-nothing per envelope: validate type/scope for every target first, show the count and first 10 conflicts, then commit. Cap 5,000 nodes per command; larger scopes chunk at 5,000 with one `batchId` and one undo group. `variable.rebind` never rewrites the variable's own alias; it rewrites references.

### 3.6 Token editing table, import/export, code sync

**Table columns:** Name, Type, Value (current theme), Modes (chips), Alias, Scope, Usage, Library, Updated. Sortable by every column; filter by category, collection, library, unused, deprecated, lint. Virtualized at 2,000 rows; sticky header; column widths persisted per user.

| Interaction | Behavior |
|---|---|
| Inline edit | `Enter`/double-click; commit on blur; one command per cell; `Esc` cancels. |
| Multi-cell paste | Tab/newline TSV; validates all cells; atomic commit or full rejection with a cell-level error report. |
| Rename | Edit the path inline. **Rename group** renames every descendant (confirmation with count); ids/keys never change. |
| Move | Drag to another collection only when resolved type and modes are compatible; otherwise disabled with reason. |
| Delete | Usage check; **Replace with…**, **Detach all**, or **Cancel**. |
| Create | `+ Token` at collection level; type-first flow; value editor per type. |
| Alias cell | Toggle literal/alias; alias picker filters to compatible types. |

| Import/export | Format | Scope | Rules |
|---|---|---|---|
| Import | Kanvas JSON (`tokens.json` projection) | Collections, modes, variables, styles, themes | Full fidelity; ids preserved when keys match, new ids otherwise. |
| Import | W3C Design Tokens (DTCG) JSON | Single mode per collection in MVP | `$type: color` → `COLOR` (sRGB); `dimension` → `FLOAT` px; alias `{path}` → `VariableAlias`; composite `shadow`/`typography` → `Style`. Unknown groups preserved in `metadata.pluginData`. |
| Import | CSS custom properties | Flat single-mode variables | `--color-bg-surface` → `color/bg/surface`; `:root` is default mode; media/theme blocks require explicit mapping. |
| Export | Kanvas JSON | All | Includes modes, themes, scopes, codeSyntax. |
| Export | DTCG JSON | Active theme only | Emits a warning that modes are flattened; aliases preserved. |
| Export | CSS custom properties | Active theme; optional dark selector | `:root { --color-bg-surface: … }`, `[data-theme="dark"] { … }`; no runtime. |

**Import flow.** File picker or drag-drop → parse → dry-run diff (added/changed/conflicting) → conflict policy **Merge** (default; keep existing on conflict) or **Replace** → commit as one `HistoryEvent` with `origin: 'IMPORT'`. Parse errors list line/column; no partial import. Cap 10,000 tokens per import.

**Sync with code.** **Sync tokens** opens the handoff panel (entry point specified in §7.4; generation details in `08-export-and-codegen.md`). It shows: last synced library version, generated artifact name, drift status (`In sync` / `N tokens changed since last export`), and **Generate**. The generated token file is read-only in the repo; Kanvas never writes to source control directly.

### 3.7 Styles vs variables

| Need | Use | Why |
|---|---|---|
| One color value that changes by theme/brand | `Variable` (COLOR) | Modes, aliasing, runtime theme switching. |
| Multi-paint fill, gradient, image fill | `Style` (FILL) | A `Variable` holds one color, not a paint stack. |
| Multi-layer shadow / elevation | `Style` (EFFECT) | Composite effect list. |
| Typography bundle (family, size, line height, spacing) | `Style` (TEXT) | One style applies all text properties. |
| Grid | `Style` (GRID) | Layout grid array. |
| Stroke width + color pair | `Variable` for each, or `Style` (STROKE) | Use a Style when the pair must travel together. |
| Component-scoped color role | `Variable` component token | Alias semantic, never primitive. |

**Composition precedence.** Node `boundVariables` > `Style.boundVariables` > literal value inside the style definition. Applying a style does not remove node-level bindings; the properties panel shows both chips and explains which wins. Editing a `Style` updates all consumers and is one `HistoryEvent`; **Detach style** copies the resolved definition to the node, then removes the `StyleRef`.

**UI affordances.** The style picker shows a preview thumbnail and library badge; the token picker shows mode values. Styles use **Apply style** / **Detach style** (`style.applyStyle` / `style.detachStyle`); variables use **Bind** / **Detach token**. Converting a style to variables (or back) is not automatic in MVP; it is a V1 migration tool.

**Style commands.** `style.createStyle`, `style.updateStyle`, `style.updateStyle`, `style.deleteStyle`, `style.applyStyle`, `style.detachStyle`. Deleting a style in use requires replace/detach; remote styles are read-only until the library update is accepted.

### 3.8 Governance: publishing, deprecation, migration, history

**Publishing tokens** uses the library publish flow in §2.8, with token-specific semver:

| Bump | Token change |
|---|---|
| Patch | Change a value in one or more modes; description; codeSyntax fix; contrast-only adjustment |
| Minor | Add variable/style/collection/mode; add scope; add alias target |
| Major | Delete or rename a token; change resolved type/category; delete a mode; change a collection's mode set; change component-token namespace |

`hiddenFromPublishing` items are excluded from the diff and the artifact, and consumers never see them. Publishing requires `publish` scope and confirmation for major; `token/*` lint errors block.

**Deprecation (schema gap, explicit).** `Variable` has no deprecation field in 02. MVP stores deprecation as library-level publish metadata keyed by `Variable.key`, surfaced as a **Deprecated** badge in the token table and update diff. `variable.deprecate` is a library operation, not a `DesignCommand`, and never writes `Variable`. Consumers see "Deprecated — migrate to X" in binding chips. Promoting this to a first-class `Variable.deprecatedAt` field requires a schemaVersion bump and is tracked in §10.

**Rename.** Bindings and aliases reference ids, so a rename does not break the document. It does change exported code identifiers: `codeSyntax` is regenerated from the new name unless manually overridden; the publish diff classifies rename as major and lists consumers whose generated CSS/JS symbol changes. **Find usages** must be reviewed before commit.

**Delete.** `variable.delete` accepts one strategy:

| Strategy | Behavior |
|---|---|
| `REPLACE` | Rewrite every binding, alias, style binding, theme override, and `SET_VARIABLE` action to a replacement token of identical type; validate scope at each target. |
| `DETACH` | Resolve the current value at each binding site, remove aliases, and remove the token. Theme overrides are dropped. |
| `BLOCK` | Reject when usage >0. |

Deletion is atomic and undoable; dangling aliases are never left behind. The dialog shows usage counts by surface and a sample of affected nodes.

**Change history.** Per-token timeline from `HistoryEvent` plus library publish records: actor, timestamp, before/after value per mode, alias change, scope change, publish version. **Rollback token** uses library rollback (§2.8); local edits are undone via history, not rollback.

**Token guardrails.**

| Action | Human editor | Agent |
|---|---|---|
| Create/update token, bind/unbind | Allowed | `SAFE` with `manage_tokens` / `STYLE_APPLY` |
| Rename token | Confirm with usage count | `CONFIRM` with `manage_tokens` |
| Delete token in use | Confirm + strategy | `CONFIRM` + `DESTRUCTIVE`; never `BLOCK` silently |
| Deprecate token | Allowed | `CONFIRM` |
| Delete collection/mode | Owner/editor + migration | `PROTECTED` unless empty |
| Publish library | `publish` scope | `CONFIRM`; major requires human approval |

### 3.9 Design system empty & error states

| State | UI | Recovery |
|---|---|---|
| No collections | "No design tokens yet" + **Create collection** / **Import** | Create or import |
| No tokens in collection | Row-level empty state | Add token |
| Alias cycle | Token row magenta; binding tooltip names cycle | Break cycle |
| Broken alias target | `Unresolved` badge | Repoint/replace |
| Token in use on delete | Blocking dialog with counts | Replace/detach |
| Mode referenced by theme on delete | List of themes | Remap modes |
| Theme missing a collection mode | Theme card shows **Incomplete**; canvas falls back to default mode | Assign mode |
| No themes | Switcher shows **Default modes**; Themes tab empty | Create theme |
| No default theme | Uses collection defaults; banner suggests setting one | Set default |
| Style in use on delete | Count + replace/detach | Resolve |
| Missing font in typography style | Fallback rendering + warning | Font resolution |
| Import parse error | Error report with line/column; no changes | Fix file |
| Publish blocked by lint | Grouped error list with jump links | Fix or exclude |
| Deprecated token used | Chip badge + tooltip | Migrate |

### 3.10 Design system acceptance criteria

1. Binding a fill to `color/bg/surface/raised` writes exactly one alias at `fills.0.color`, renders theme-reactive, and rejects a `FLOAT` token with a typed error.
2. Switching `light` → `dark` updates 5,000 bound nodes in ≤100ms p95 with no document mutation and no `HistoryEvent`.
3. Creating a component token that aliases a primitive is rejected by `token/component-alias-primitive`; a semantic token aliasing the same primitive is accepted.
4. Renaming a token leaves every binding and alias byte-identical by id; the publish diff reports a major change and lists code syntax impact.
5. Deleting a used token with `REPLACE` rewrites all bindings/aliases/actions atomically; no `VariableAlias` points to the deleted id afterward.
6. `variable.extract` creates a token with the exact resolved RGBA and binds it in one undoable action.
7. Theme preview never writes to the document or history; `Esc` restores the previous theme within one frame.
8. DTCG import of 1,000 tokens produces 1,000 variables with aliases preserved; re-export is structurally equivalent (ignoring ids/order).
9. A theme missing a mode falls back to the collection default and shows **Incomplete**; the player resolves identically.
10. An agent without `manage_tokens` receives a permission error before any command is applied; an agent deleting a used token requires confirmation and records an audit entry.

### 3.11 Agent-relevant affordances — design system

| Capability | Level | Constraint |
|---|---|---|
| Create/bind tokens while designing | `SAFE` | Must use existing semantic tokens when a match exists; token compliance target ≥0.9 (00 §8.2). |
| Update token value | `SAFE` | One mode per command; reports affected node count. |
| Rename token, change scope | `CONFIRM` | Usage list shown. |
| Delete token in use, delete mode/collection | `CONFIRM` / `PROTECTED` | Destructive scope + owner for collection deletion. |
| Change default theme | `CONFIRM` | Affects every unthemed surface. |
| Publish library | `PROTECTED` from agent | Requires human publisher; `publish` scope alone is insufficient for tokens. |

---

## 4. Assets

### 4.1 Asset library UI

**Assets → Library** (left rail, `⌥3`). Sub-tabs: **All**, **Images**, **Icons**, **Illustrations**, **Fonts**. Grid/list toggle; folder tree (one level of nesting per folder, max 5 levels); tag chips; search across name, tags, and folder; filters: kind, library, unused, license missing, uploaded by, date. Sort: recent, name, size, usage.

| Element | Behavior |
|---|---|
| Card | Thumbnail, name, dimensions, format, size, usage count, library badge; hover shows **Place**, **Replace selection**, **More**. |
| Multi-select | Bulk tag, move, delete (blocked if used), download originals, **Replace globally**. |
| Usage panel | Node ids grouped by page/frame; click selects and frames; styles using the asset are listed separately. |
| Dedupe | Uploading identical bytes returns the existing `Asset` (same `hash`), merges tags, adds usage; toast **Already in library**. |
| Empty state | Drop zone + **Upload** + **Browse team library** (V1). |

Primary commands: `asset.upload`, `asset.register`, `asset.update`, `asset.update`, `asset.update`, `asset.delete`, `asset.replace`, `asset.insert`, `asset.update`.

### 4.2 Upload and ingest

| Input | Accepted | Limits | Result |
|---|---|---|---|
| Images | PNG, JPEG, WebP, AVIF, GIF (first frame only) | ≤50MB, ≤8192×8192, total pixels ≤40MP | `Asset.kind: 'IMAGE'`; animated inputs show a warning and store frame 1. |
| SVG | `image/svg+xml` | ≤10MB, ≤20,000 path commands, depth ≤32 | `Asset.kind: 'SVG'` or `'ICON'`/`'ILLUSTRATION'` by user choice; parsed to `VectorPath` cache. |
| Fonts | TTF, OTF, WOFF2 | ≤20MB | `Asset.kind: 'FONT'`; registered by family/style. |
| Video | MP4/WebM | Out of MVP | Rejected with roadmap notice. |

**Pipeline (synchronous stages with progress):** receive → SHA-256 → dedupe lookup → MIME sniff (never trust extension) → decode/validate → sanitize (SVG) → optimize display derivative → thumbnail (256px WebP) → metadata (dimensions, license prompt) → commit `Asset` + storage object. Failure at any stage leaves no orphan record; partial storage objects are garbage-collected within 24h. Uploads are capped at 20 concurrent per file; queue beyond that.

**Naming.** Default from filename, extension stripped, spaces → `-`, max 120 chars; duplicates get ` 2`. Renaming never changes `hash` or `storageKey`.

### 4.3 SVG sanitization and import mapping

Sanitization runs before parsing. Rejected content: `<script>`, `<foreignObject>` (unless **Rasterize** chosen), event-handler attributes (`on*`), external `href`/`xlink:href`, `javascript:` and `file:` URLs, `<iframe>`, `<object>`, `<embed>`, CSS `url()` to remote origins, and entity expansion beyond 1MB. Allowed data URIs: `data:image/png|jpeg|webp;base64` only.

| SVG construct | Mapping | Fidelity |
|---|---|---|
| `<path>` | `VECTOR` with `VectorPath[]` | Cubic/quadratic commands preserved; arcs converted to cubics. |
| `<rect>` (no rx/ry) | `RECTANGLE` | Exact. |
| `<rect rx/ry>` | `RECTANGLE` with `cornerRadius` | Exact for uniform radius; elliptical corners converted to path. |
| `<circle>`, `<ellipse>` | `ELLIPSE` | Exact. |
| `<line>`, `<polyline>`, `<polygon>` | `LINE` / `VECTOR` | Exact. |
| `<g>` | `GROUP` | Transform preserved; unsupported blend modes fall back to `NORMAL` with warning. |
| `<use>` | Expanded copy | Referenced `<symbol>`/`<defs>` resolved; cycle rejected. |
| `<text>` | Outlined `VECTOR` | **Font is not imported**; warning "Text converted to outlines". |
| `<image>` | `IMAGE` node + uploaded `Asset` | Embedded data URIs extracted; remote URLs rejected. |
| `<clipPath>`, `<mask>` | `isMask` / clip group | Supported; complex masks may rasterize with warning. |
| `<filter>` | Nearest supported `Effect` or rasterize | `feGaussianBlur` → `LAYER_BLUR`; others rasterized at 2x. |
| `<linearGradient>`, `<radialGradient>` | `GRADIENT_*` paint | Stops/transform mapped; focal gradients rasterized. |

Options on import: **Editable vectors** (default), **Flatten to single vector**, **Rasterize** (PNG at 2x). `currentColor` maps to `color/fg/default` if it exists, else black, with a warning. IDs are namespaced per asset to prevent collisions. Imported SVG creates one root `GROUP` or `FRAME` on the current page, centered in the viewport, with a 0.5px alignment snap.

### 4.4 Icon sets, illustrations, optimization

- **Icons.** Name convention `icon/{name}`; optional `icon/{name}/{size}` variants. Tag `icon`. Inserting an icon creates a `VECTOR`/`GROUP`; fill is bound to `color/fg/default` by default. Icon sets imported from a folder batch keep folder name as tag.
- **Illustrations.** Tag `illustration`; recommended max 4096px; stored as SVG when vector, otherwise optimized raster. License fields are required for `EXTERNAL` library assets.
- **Image optimization.** Original bytes are preserved for export fidelity. `Asset.optimized` stores the display derivative: longest edge ≤4096px, AVIF quality 50 when supported, otherwise WebP quality 62. `thumbnailKey` is a 256px WebP. Derivatives are generated on upload and on replace; they never upscale. Images >4096px display the derivative but export from the original up to 8192px. A file with >500 assets warns at 400 and blocks upload at 500 in MVP.

### 4.5 Replace globally, usage, license

**Replace globally** (`asset.replace`) swaps `assetId` at every usage: `ImageNode.assetId`, `ImagePaint.assetId`, `Style.definition` image paints, and component masters. Crop transforms, filters, scale modes, and rotation are preserved when aspect ratio differs within 10%; otherwise the node resets to `FIT` and shows a warning. One `HistoryEvent`; undo restores every usage. Replace is blocked for fonts (use `font.replace`) and for assets in a read-only library.

**Usage** is maintained by the engine from `Asset.usage.nodeIds` plus style/component references; the panel distinguishes direct node usage from indirect style usage. Deleting an asset with usage >0 requires **Replace first** or **Detach all**; unused assets delete immediately with undo (trash 30 days). `asset.delete` on a remote library asset is disabled.

**License metadata** (`asset.update`): name, URL, attribution. Required for uploads flagged **External / stock**; optional otherwise. Missing license on a team-library asset shows `asset/missing-license` and blocks publish, not local use. Attribution text is exported in the handoff README and shown in the asset details.

### 4.6 Placing, crop, filters, rotation

| Interaction | Behavior | Command |
|---|---|---|
| Drag from library | Drops at pointer; default natural size clamped to 512px longest edge; `Shift` preserves aspect. | `asset.insert` |
| Click-to-place | Armed placement tool stamps one asset per click; `Esc` exits; `Alt` stamps centered. | `asset.insert` |
| Replace selection | Drop onto a selected image/shape, or **Replace selection** on the card; preserves transform. | `asset.replace` |
| Scale mode | `FILL`, `FIT`, `CROP`, `TILE` in the fill section; per-image-paint. | `image.setScaleMode` |
| Crop | `CROP` shows a crop rectangle with thirds guides; drag handles; `Enter` commits; `Esc` cancels. Crop is stored in `cropTransform`. | `image.setCrop` |
| Filters | Exposure −5..+5, contrast −100..100, saturation −100..100, temperature −100..100, tint −100..100, highlights/shadows −100..100; reset per filter and all. | `image.setFilters` |
| Rotation | 0/90/180/270 only; preserves crop bounds; UI shows rotate-left/right buttons. | `image.rotate` |

`FILL` crops overflow, `FIT` letterboxes, `CROP` uses the stored transform, `TILE` repeats at `tileScale` (0.1–10). Changing scale mode never destructively resamples the asset. Filters are non-destructive and exported.

### 4.7 Fonts as assets

**Upload/registration.** Fonts are uploaded in the Assets → Fonts tab or drag-dropped; family and style are parsed from the file and MUST be confirmed. Registration is workspace-scoped by default; a file may embed a font asset reference. Licensing warning: "You must have a license to use this font in shared files" with a required checkbox; the license name/URL is stored on the `Asset`. Variable fonts register as one family with named instances.

**Missing font resolution.** A banner lists missing families with usage counts (nodes, styles, masters, instances). Flow: **Choose replacement** (family/style) → preview selection/file → **Apply**. Mapping is stored as a UI resolution record for the session and applied via `font.replace`, which rewrites `textStyle.fontFamily`/`fontStyle` on affected nodes and styles, preserving size/line height. Per-node override is available from the text properties panel. Until resolved, text renders with the fallback stack (`Inter`, then `system-ui`) and `fontState: 'LOADING'`; layout uses fallback metrics and may shift. Publishing a master with a missing font is a warning; exporting is allowed with a warning baked into the handoff notes.

### 4.8 Asset permissions, errors, acceptance

| Action | Human | Agent |
|---|---|---|
| Upload/import asset | Editor | `CONFIRM` with `ASSET_MANAGE`; license required |
| Rename/tag/move | Editor | `SAFE` |
| Replace globally | Editor + confirm | `CONFIRM` with affected count |
| Delete used asset | Editor + replace/detach | `CONFIRM` + `DESTRUCTIVE` |
| Delete remote library asset | Disabled | `PROTECTED` |
| Register font | Editor + license checkbox | `CONFIRM` |

| State | UI | Recovery |
|---|---|---|
| Unsupported SVG feature | Import report lists element/line; asset imports with supported subset or is rejected | Choose rasterize/flatten |
| SVG too complex | Error with command/limit counts | Simplify or rasterize |
| Duplicate upload | Toast, existing asset selected | Use existing |
| Image too large | Error before upload | Resize |
| Missing asset (dangling) | Magenta checkerboard + `Missing asset` badge | Relink/replace |
| Missing license | Amber badge; publish blocked for library | Add license |
| Font parse failure | Error with reason | Convert to WOFF2/TTF |
| Font missing | Banner + fallback rendering | Replace mapping |

**Acceptance criteria.**

1. Uploading identical bytes twice yields one `Asset` with one `hash` and merged tags; usage count increments.
2. SVG import strips scripts/handlers/external refs; a fixture with 12 unsafe constructs imports zero executable content.
3. Replacing an asset globally preserves crop/filters on ≥90% aspect-compatible usages and is undoable in one event.
4. A 12MP JPEG ingests to a 256px thumbnail and a ≤4096px display derivative in ≤3s p95 on a 10Mbps connection.
5. `FILL`/`FIT`/`CROP`/`TILE` render correctly after save/reload with identical crop transforms.
6. Missing font resolution rewrites all selected usages, preserves font size, and leaves no node with the missing family.
7. A used asset cannot be deleted without replace/detach; the dialog shows exact usage counts.
8. Agent upload without license metadata is rejected before storage commit.

### 4.9 Agent-relevant affordances — assets

| Capability | Level | Constraint |
|---|---|---|
| Search/place asset, set scale mode, filters, crop | `SAFE` | Must preserve aspect unless explicitly asked. |
| Upload generated image/SVG | `CONFIRM` | License/attribution required; file-size limits apply. |
| Replace globally | `CONFIRM` | Shows affected node count; one undo group. |
| Delete asset, register font | `CONFIRM` / `PROTECTED` | Fonts require human license acceptance. |

---

## 5. Prototyping

### 5.1 Prototype mode UI

Toggle **Prototype** at the top-right or press `⌥P`. Non-interactive nodes dim to 60%; interactive nodes get a 1px dashed outline; connectors render as 1.5px curved lines in the prototype accent color. The right panel shows: flow selector, selected interaction list, trigger/action editor, transition, overlay, and **Present**. Exiting preserves selection. Prototype edits are document mutations with `origin: 'USER' | 'AGENT'` and normal undo.

### 5.2 Flows and starting points

A `Flow` is a named entry point (`startNodeId`) on a `Page`. Creating a start point sets `FrameNode.isPrototypeStart: true` and creates the `Flow` (`prototype.setFlow`); deleting the last flow for a frame clears the flag. A frame MAY start multiple flows. Start nodes MUST be top-level `FRAME` or `COMPONENT` nodes; nested frames are rejected with a picker that offers the nearest valid ancestor.

| Control | Behavior |
|---|---|
| Flow list | Per page; name, start frame, reachable frame count, warning count. |
| Create | Select frame → **+ Flow**; default name `Flow 1`; inline rename. |
| Rename/delete | `prototype.setFlow` / `prototype.deleteFlow`; deleting keeps interactions, clears start flag. |
| Present | Dropdown lists flows; default is selected flow, else current frame as an ephemeral flow. |
| Share | Link includes `?flow=fl_…`; opening starts at that flow. |

### 5.3 Linking

Drag from the circular connector handle on a selected node to a destination frame. Valid destinations are `FRAME`/`COMPONENT` roots; dropping elsewhere cancels. The first link creates a default `Interaction`: `ON_CLICK`, `NAVIGATE` to destination, `INSTANT` transition. Multiple interactions per node are allowed and listed in `order`; drag to reorder. A node can link to itself (validation warning for input-less loops). `prototype.link`, `prototype.removeLink`, `prototype.updateLink`, `prototype.reorderInteraction`.

Connector selection highlights both endpoints; clicking a connector opens the interaction editor. Deleting a destination frame leaves the interaction with a broken reference and `prototype/broken-link`; it is never silently removed.

### 5.4 Triggers

All trigger types from 02 §12 are supported. Only one interaction fires per input event.

| Trigger | Fires when | Defaults / limits | Cancellation |
|---|---|---|---|
| `ON_CLICK` | Pointer down and up on the same hit target, movement <4px | No duration limit; touch and mouse | Pointer leaves target or movement ≥4px |
| `ON_PRESS` | Pointer down on hit target | Immediate; no repeat | Pointer up does not undo the action |
| `ON_HOVER` | Pointer remains over target for `delayMs` | `delayMs` 0–5000, default 0 | Pointer leaves before delay; leaving does not reverse the action |
| `ON_MOUSE_ENTER` / `ON_MOUSE_LEAVE` | Pointer enters/leaves target bounds | No delay | — |
| `ON_DRAG` | Pointer moves ≥`thresholdPx` in allowed direction while pressed | `thresholdPx` 1–200, default 10; `direction` LEFT/RIGHT/UP/DOWN/ANY | Pointer up before threshold; wrong direction |
| `ON_SCROLL` | Wheel/trackpad delta ≥`thresholdPx` in direction | `thresholdPx` 1–500, default 24 | Scroll ends; one fire per gesture, re-arms after 300ms idle |
| `ON_KEY` | Keydown matches `key` and exact modifier set | Key case-insensitive (`Enter`, `Escape`, `ArrowLeft`, single chars); no key repeat | Keyup; focus outside player |
| `AFTER_TIMEOUT` | Frame has been settled for `timeoutMs` | 0–600000, default 1000; 0 executes next frame (≥16ms) | Frame exit, navigation, overlay close, restart |

**Hit testing.** Deepest visible, non-locked node under the pointer wins. Walk ancestors until a node with a matching trigger is found; an ancestor does not also fire if a descendant fired. Within one node, interactions sort by `order` ascending, then `id` lexicographic. Disabled interactions (`enabled: false`) are skipped. Nodes with `visible: false` never hit-test. Overlay content hit-tests above the base frame.

**Keyboard precedence.** `Escape` closes the top overlay, then navigates back if the player chrome has focus; a prototype `ON_KEY` for `Escape` on the focused node runs first. `Tab`/`Shift+Tab` move focus among interactive hotspots; `Enter`/`Space` fire `ON_CLICK`. Arrow keys scroll the frame unless an `ON_KEY` interaction claims them.

### 5.5 Actions

Actions inside one `Interaction` execute in array order. `NAVIGATE`, `BACK`, and `CLOSE_OVERLAY` are **terminal**: remaining actions in that list are skipped and the validation panel warns. `NODE_ACTIONS` nests a list.

| Action | Runtime effect | Constraints | Persistence |
|---|---|---|---|
| `NAVIGATE` | Pushes current frame onto history, renders destination | Destination frame/component must exist; `resetScroll` default `true` | Session only |
| `BACK` | Pops history; restores prior frame and scroll | Empty stack: no-op with toast; closes player only if no history and no overlays | Session |
| `CLOSE_OVERLAY` | Closes top overlay | No-op if none | Session |
| `OPEN_LINK` | Opens URL | Scheme allowlist `https`, `http`, `mailto`, `tel`; `javascript:`/`data:`/`file:` rejected; popup-blocked shows copyable URL | External |
| `SCROLL_TO` | Scrolls current frame to target | Target must be descendant of current frame; `behavior` SMOOTH/INSTANT; offset ±10000 | Session |
| `SET_VARIABLE` | Sets session variable value or collection mode | Type must match `resolvedType`; `modeId` must belong to the variable's collection | Session |
| `CHANGE_VARIANT` | Sets a VARIANT property on an instance in the current frame | Target must be an `INSTANCE` with that property; unknown value falls back to default + warning | Session |
| `SET_STATE` | Sets `visible` or `opacity` on a node | Property allowlist only; value type checked; node may be outside current frame (state retained) | Session |
| `NODE_ACTIONS` | Executes nested actions | Depth ≤8; total actions per interaction ≤32 | Session |
| `AI_GENERATE` | Requests a runtime agent generation | V2; consent + budget (§5.15) | External run |
| `AI_VARIANT` | Requests N variants of a frame | V2; `variants` 1–6, default 3 | External run |

### 5.6 Transitions and easing

| Type | Default duration | Behavior | Overlay allowed |
|---|---|---|---|
| `INSTANT` | 0ms | Cut | Yes |
| `DISSOLVE` | 200ms | Cross-fade outgoing/incoming | Yes |
| `SMART_ANIMATE` | 300ms | Match and tween (§5.7) | No (falls back to DISSOLVE) |
| `MOVE_IN` / `MOVE_OUT` | 250ms | Destination slides in from / source slides out to direction | Yes (in only) |
| `PUSH` | 300ms | New frame pushes old out | No |
| `SLIDE_IN` / `SLIDE_OUT` | 250ms | Slide with parallax underlay | Yes (in only) |

Duration `0–10000ms`; `0` is equivalent to INSTANT. Easing presets: `LINEAR (0,0,1,1)`, `EASE_IN (0.42,0,1,1)`, `EASE_OUT (0,0,0.58,1)` default, `EASE_IN_OUT (0.42,0,0.58,1)`, `GENTLE (0.25,0.1,0.25,1)`, `QUICK (0.4,0,0.2,1)`, `BOUNCY (0.34,1.56,0.64,1)`, `SLOW (0.16,1,0.3,1)`. Custom cubic accepts x1/y1/x2/y2 with x in 0–1; spring accepts stiffness 1–1000, damping 1–100, mass 0.1–10. Invalid values are rejected before commit.

During `TRANSITIONING`, pointer triggers are ignored; queued pointer events are dropped on settle. `AFTER_TIMEOUT` timers for the incoming frame arm at settle. `ON_KEY`/`Escape` remain active. Only one transition runs at a time.

### 5.7 Overlays, scroll, smart animate

**Overlays.** A `NAVIGATE` action with `Interaction.overlay` opens the destination above the current frame instead of pushing history.

| Setting | Behavior |
|---|---|
| Position | 9 anchors or `MANUAL`; manual position clamped so ≥24px remains visible. |
| Background | `NONE` or `SOLID` with RGBA; default `rgba(0,0,0,0.4)`. |
| Dismiss outside | When true, click on backdrop closes; when false, only `CLOSE_OVERLAY`/`Escape`. |
| Keep scroll | Preserves base-frame scroll position when the overlay opens/closes. |
| Stack | Max 8; each overlay traps focus; backdrop click closes only the top if enabled. |

Overlay transitions limited to DISSOLVE/MOVE_IN/SLIDE_IN; SMART_ANIMATE and PUSH are invalid and blocked with an explanation. Overlay size is the destination frame's own size; it does not auto-resize to content.

**Scroll.** Frames scroll when content exceeds bounds and `clipsContent` is true. Wheel/trackpad scrolls the current frame; nested scroll containers consume first until their end, then chain to the parent. Scroll offsets are stored per frame in the navigation stack; `NAVIGATE` resets to `(0,0)` unless `resetScroll: false`; `BACK` restores the previous offset. `ON_SCROLL` fires once per gesture after `thresholdPx`; `SCROLL_TO` animates 300ms or jumps.

**Smart Animate.** Matching follows 02 §12 exactly: identical id → `componentKey` → name + type → size similarity >0.85. Matched pairs tween transform, opacity, and solid→solid fills; unmatched nodes cross-fade. UI exposes duration, easing, and direction only; the match list is a read-only diagnostic showing each pair and the rule used. There is no manual pairing in MVP. Mismatched paint types cross-fade rather than tween. Reduced motion maps Smart Animate to DISSOLVE 0ms.

### 5.8 Pages, back behavior, variables, conditionals

- **Multi-page navigation.** Destinations may be on another page. The player loads the destination page on demand (02 §17 lazy pages) and shows a 150ms loading state if it is not cached. `BACK` restores the prior page, frame, and scroll offset.
- **History.** Stack max 64 frames. Exceeding it drops the oldest entry and shows a one-time toast. Browser back mirrors `BACK`; browser forward is not supported.
- **Variables.** `SET_VARIABLE` writes session state keyed by variable id. With `modeId`, it switches that collection's session mode; every bound node re-resolves within 100ms. Without `modeId`, it sets the variable value for the session regardless of mode (highest precedence over mode value, not persisted). Invalid type/scope skips the action and records a runtime warning. Theme overrides are not modified.
- **Change variant.** Applies to the target instance's `componentProperties[propertyKey]`; the property must be `VARIANT`. It re-renders the mirrored subtree and re-arms timeouts on newly visible nodes. It does not write the document.
- **Conditionals.** MVP has **no conditional logic**: `Interaction` has no condition field in schema v1, and this document does not add one. Conditional behavior is modeled by (a) putting interactions on variant-specific nodes, or (b) using different frames per state. V1 adds only variant/state-based branching; a general expression language requires a schema change and is **V2** (§10).

### 5.9 Runtime state machine

`PrototypeRuntimeState` is implementation-local, never persisted: `SETTLED | ARMED | FIRING | TRANSITIONING | BLOCKED`.

| State | Meaning | Exits |
|---|---|---|
| `SETTLED` | Frame rendered, no pending input | Pointer event → `ARMED`/`FIRING`; timeout due → `FIRING` |
| `ARMED` | Hover/drag/scroll threshold accumulating | Threshold met → `FIRING`; cancel → `SETTLED` |
| `FIRING` | Executing an action list | Actions complete → `TRANSITIONING` or `SETTLED` |
| `TRANSITIONING` | Transition playing | Animation end → `SETTLED`; input ignored except Escape |
| `BLOCKED` | Invalid runtime action (missing destination, bad variable) | Warning dismissed → `SETTLED` |

**Evaluation pipeline (pure reducer).**

1. Normalize event: map pointer/touch/key/scroll to canvas coordinates; drop repeats; ignore if `TRANSITIONING` (except key).
2. Hit-test deepest eligible node, then ancestors; collect interactions whose trigger matches the event.
3. Sort by `Interaction.order` ascending, then `id`; take exactly one.
4. Apply trigger-specific arming: hover delay, drag/scroll threshold, timeout clock. Cancel prior arm if pointer leaves or frame changes.
5. Execute actions sequentially. Enforce total action budget 32 and `NODE_ACTIONS` depth 8.
6. Terminal action (`NAVIGATE`/`BACK`/`CLOSE_OVERLAY`) stops the list; push/pop state; compute transition.
7. Run transition; on settle, re-arm `AFTER_TIMEOUT` for the current frame and reset hover/scroll gesture state.
8. Record a runtime warning if an action was skipped; never write document state.

**Loop and budget guards.** Max 100 navigations per 60s per session; exceeding shows **Prototype appears to loop** and pauses. An `AFTER_TIMEOUT` loop (A→B→A with no input) is blocked at the 10th cycle in the player and reported as an error in validation. `SET_VARIABLE`/`CHANGE_VARIANT`/`SET_STATE` do not count as navigations.

```
reduce(state, event, doc, variables):
  if state.phase == TRANSITIONING and event.kind != KEY: return state
  hit = hitTest(event, state.frame, state.overlayStack)
  interaction = selectInteraction(hit, event)
  if interaction == null: return armOrCancel(state, event, hit)
  state = fire(interaction.actions)      # terminal action stops list
  return transitionTo(state, interaction.transition)
```

### 5.10 Prototype player

| Capability | Behavior |
|---|---|
| Device frames | None, iPhone 15 (393×852), Pixel 8 (412×915), iPad (1024×1366), Desktop (1440×900), custom; frame chrome is decorative and excluded from screenshots unless enabled. |
| Scaling | Fit (default), 100%, Fill; keyboard `+`/`-`; remembers per file. |
| Hot reload | Design changes arrive ≤500ms via sync; current frame preserved if it still exists; if deleted, falls back to nearest history entry, else flow start, with a banner. Variable/session state is preserved. |
| Share link | `?flow=fl_…&frame=n_…&theme=th_…`; respects `FileMeta.sharing`; no link if visibility is `PRIVATE` and viewer lacks access. |
| Presentation | Fullscreen, hides editor chrome, `Esc` exits fullscreen then player; `R` restarts flow and resets variables; `←`/`→` navigate history when no `ON_KEY` claims them. |
| Keyboard/a11y | Focus ring visible; `Tab` cycles hotspots; `Enter`/`Space` activate; navigation announced via `aria-live` ("Navigated to Login"); player chrome labelled; comment/share buttons reachable. |
| Reduced motion | `prefers-reduced-motion`: Smart Animate/MOVE/PUSH/SLIDE become DISSOLVE 0ms; cursor and hover animations disabled; timers continue. If any `AFTER_TIMEOUT` >5s exists, a **Pause auto-advance** control is shown (WCAG 2.2.2). |
| Error chrome | Broken destination shows an inline "Destination missing" card with a link back; viewers see no stack traces. |

### 5.11 Preview mode on canvas

**Preview current frame** (`⌘Enter`) spotlights the selected frame: all other canvas content dims to 20%, the frame is centered at Fit, and interactions are live. Toolbar: device, scale, restart, comments, exit. Editing is disabled except comment pins; `Esc` exits. This is an ephemeral flow, not a `Flow` entity, and is not shareable. **Present** opens the full player.

### 5.12 Validation

| Rule ID | Severity | Condition | Runtime behavior |
|---|---|---|---|
| `prototype/broken-link` | Error | `Action.NAVIGATE.destinationId` missing/deleted | Interaction skipped; editor warning |
| `prototype/unreachable` | Warning | Frame not reachable from any flow and not a start | Listed; player can still start there |
| `prototype/dead-end` | Warning | Frame has no outgoing interaction and is not terminal by intent | Listed |
| `prototype/auto-loop` | Error | Cycle with only `AFTER_TIMEOUT`/no user input | Blocked at 10 cycles |
| `prototype/loop` | Warning | User-input cycle | Allowed; budget guard |
| `prototype/missing-destination` | Error | Empty or non-frame destination | Action skipped |
| `prototype/invalid-overlay` | Error | Overlay transition PUSH/SMART_ANIMATE, or non-frame destination | Falls back to DISSOLVE |
| `prototype/invalid-variable` | Error | `SET_VARIABLE` type/mode mismatch | Action skipped |
| `prototype/invalid-variant` | Error | `CHANGE_VARIANT` target not an instance or property not VARIANT | Action skipped |
| `prototype/duplicate-key` | Warning | Two `ON_KEY` interactions on same node with identical key/modifiers | First by order wins |
| `prototype/terminal-actions` | Warning | Actions after terminal action | Skipped |
| `prototype/empty-flow` | Warning | Flow start has no outgoing interaction | Listed |

The **Prototype warnings** panel groups by severity, shows node name/page, and offers **Select** and **Fix** (open the relevant editor). Broken links also render a red connector. Validation runs on open, on every prototype mutation (debounced 300ms), and before share-link creation; share links are allowed with warnings but blocked with errors until the owner chooses **Share anyway**, which records an audit entry.

### 5.13 AI actions in prototypes (V2)

`AI_GENERATE` and `AI_VARIANT` are runtime agent invocations, not document mutations. They are disabled by default per file via `GuardrailPolicy.allowNetwork` and an explicit prototype AI toggle.

| Aspect | Requirement |
|---|---|
| Consent | Before the first AI action per session, a modal names the agent, prompt, target frame, estimated cost, and latency; **Run** is required. Viewers with `VIEW` cannot trigger. |
| Permissions | Requires an authenticated session with `write` scope and `PROTOTYPE_LINK` or `DESIGN_CREATE` capability; anonymous/guest viewers are blocked. |
| Cost/latency | Estimate shown; hard cap `maxCostUsdPerRun` and a session cap of $1.00 unless the owner raises it. Latency states: `QUEUED`, `GENERATING`, `PREVIEW_READY`, `FAILED`, `CANCELLED`; timeout 60s with cancel. |
| Result | Generated output appears in a sandbox preview, never directly in the document. **Apply** creates normal nodes/frames via standard commands (`origin: 'AGENT'`) with a run id; **Discard** removes the sandbox. |
| Guardrails | Rate limit 3 actions/5min/session; no network if `allowNetwork: false`; protected masters never targets; generated content is linted for tokens/contrast before Apply. |
| Failure | Inline error with retry; no partial document change; audit entry records the attempt, cost, and denial reason. |

### 5.14 Prototype acceptance criteria

1. `prototype.link` creates one interaction with `ON_CLICK`+`NAVIGATE`+`INSTANT`; presenting the flow navigates in ≤100ms p95 after click.
2. All 9 trigger types fire under the specified thresholds; movement ≥4px suppresses `ON_CLICK`; drag below threshold does not fire.
3. Terminal actions stop the action list; `NODE_ACTIONS` beyond depth 8 is rejected at authoring time.
4. Overlay position, backdrop dismissal, `CLOSE_OVERLAY`, and `Escape` behave identically in editor preview and shared player.
5. `SET_VARIABLE` with `modeId` re-resolves bound nodes in ≤100ms and is reset by Restart; no `HistoryEvent` is created.
6. Smart Animate matches by the 02 order, tweens only supported properties, and falls back to cross-fade for unmatched nodes.
7. Reduced-motion mode converts spatial transitions to instant dissolve and exposes pause for auto-advance >5s.
8. Hot reload preserves the current frame and variable state; a deleted frame falls back with a visible banner, not a blank screen.
9. Validation detects broken links, auto-loops, invalid variables, and unreachable frames; errors block share link unless explicitly overridden and audited.
10. AI actions cannot run without consent, never mutate the document before Apply, and respect rate/cost limits.

### 5.15 Agent-relevant affordances — prototyping

| Capability | Level | Constraint |
|---|---|---|
| Create/update links, flows, transitions, overlays | `SAFE` | `PROTOTYPE_LINK`; must not overwrite an existing interaction without showing the diff. |
| Set variables/variants in runtime actions | `SAFE` | Type-checked; no conditionals invented. |
| Delete a flow or all interactions on a frame | `CONFIRM` | Count shown; undoable. |
| Trigger `AI_GENERATE`/`AI_VARIANT` at runtime | `CONFIRM` | V2; cost/latency consent; never on protected frames. |
| Edit protected prototype masters | `PROTECTED` | Master guardrails apply. |

---

## 6. Collaboration (Human)

### 6.1 Presence

| Element | Behavior |
|---|---|
| Cursor | Rendered in actor color with a 2px white outline; throttled to 30Hz client / 20Hz network; interpolated over 80ms; name label after 120ms hover or 2s movement. |
| Selection | 1.5px outline + 8% fill in actor color; hidden while dragging; multi-select shows one bounding box per actor. |
| Avatars | Top-right stack, max 5 + `+N`; human and agent avatars visually distinct (agent has cursor status chip). |
| Color | Assigned deterministically from a 12-color identity palette by hashing `actorId`; agents use `Agent.color`; collisions resolved by lightness offset. |
| Idle | Cursor dims after 10s, avatar dims after 60s, hidden from canvas after 5min but remains in the avatar list; `PresenceState` expires after 30s silence (02 §14). |
| Follow | Click avatar → camera follows their viewport at 30fps; any pan/zoom/selection input exits; banner `Following Maya — Esc to stop`. |
| Offscreen | Edge chevron with actor color when cursor is outside viewport; click jumps to it. |
| Ordering | `PresenceState.seq` is monotonic per actor; receivers drop out-of-order updates. |

Presence is ephemeral and never written to Postgres or history. Agents can be followed the same way; their cursor exposes `AgentRuntimeState` in the label.

### 6.2 Comments

| Operation | Behavior | Command |
|---|---|---|
| Pin | `C` arms comment tool; click over a node anchors `anchorNodeId` + stores canvas `position`; click empty canvas stores `position` only. `Alt` pins without opening the composer. | `comment.create` |
| Thread | First comment creates `CommentThread`; replies append `Comment` with `parentCommentId`; nesting displays 3 levels then flattens with `Replying to …`. | `comment.reply` |
| Mentions | `@` autocompletes humans and agents. A human mention notifies; an agent mention creates an `AgentRun` with the comment body + thread context (02 §14). Agent mention shows scope/cost confirmation before the run. | `comment.create` / `comment.reply` |
| Resolve/reopen | Resolver recorded in `resolvedBy/resolvedAt`; resolved pins hide unless filter **Resolved** is on. | `comment.resolve` / `comment.reopen` |
| Edit/delete | Author may edit within 15min (shows `edited`); delete is soft and leaves `[deleted]` in the thread. | `comment.edit` / `comment.delete` |
| Move pin | Drag pin; node-anchored pins stay attached to the node; canvas pins stay at coordinates. | `comment.move` |
| Reactions | Emoji subset (`👍 ✅ 👀 ❤️ 🚀`), one per actor per emoji. | `comment.react` |
| Filters | All / Open / Resolved / Mentions me / My threads; search body. | UI state |
| Prototype comments | Available in player with `COMMENT` or higher; anchored to source frame node; stored in the frame's canvas coordinates; canvas-only pins are disabled in player. | `comment.create` |
| Attachments | Up to 5 image assets per comment; stored as `Asset`; thumbnails inline. | `asset.upload` |

Comments render markdown subset: bold, italic, inline code, links (`https` only), lists, code blocks. Max 10,000 chars. Offline comments queue locally and show **Sending…**; failure preserves draft. Notification rules: mention/reply always; resolve notifies author; new thread notifies frame owner only if they subscribed. All notifications appear in the inbox and as a badge; email is opt-in (V1).

### 6.3 Multiplayer editing semantics (Yjs CRDT)

Reference: `07-state-model.md` §6 defines the sync protocol, Yjs document layout, and provider. This section defines UX obligations.

| Scenario | Semantics | UX |
|---|---|---|
| Concurrent same-property edit | Property-level register converges deterministically by Yjs client ordering; one write wins | Losing editor sees a 6s toast `<Name> changed this after you`; their value remains in `HistoryEvent` and can be restored from history |
| Concurrent drag of same node | Transform written as one atomic tuple `(x,y,w,h,rotation,scale)`; no partial merges | Both see a short snap to the winning value; no oscillation; drag ends cleanly |
| Node deleted while another edits | Delete wins; pending edits on that node reject with `NODE_DELETED` | Editor sees `Node deleted by <Name>` + **Restore** (trash 30 days); restore does not auto-reapply rejected edits |
| Selection loss | Deleted/hidden nodes are removed from remote selections silently | Local selection prunes; undo restores selection if available |
| Same node, different properties | Merge independently; no conflict | Both indicators show |
| Agent + human | Agent reservation (05) is a soft lock on a subtree; human grab triggers takeover | `Claude is editing` chip; takeover yields agent, never blocks human |
| Human + human | No hard locks | `Maya is editing` chip from `isTyping`/`isDragging`/recent selection; advisory only |
| Undo | Per-actor undo stack; undo never removes another actor's changes | `⌘Z` undoes your last event; agent runs have separate **Undo run** |
| Orphaned structural change | Reparent/delete races resolve by document invariants; invalid commands reject | Warning toast with affected count; no silent repair beyond `graph` validator |

Conflicts that cannot merge at the property level (delete vs edit, reparent vs delete) open **Review changes** listing the losing operation, actor, timestamp, and **Restore mine** where valid. Nothing is silently discarded.

### 6.4 Version history

| Feature | Behavior |
|---|---|
| Timeline | Reverse-chronological, grouped by day; shows actor (human/agent), label, changed node count, and origin. Filter by actor and origin. |
| Named version | **Save version** prompts for name + optional description; creates `Version` with `revision` and snapshot. Auto versions are marked `auto`. |
| Preview | Opens read-only snapshot in canvas with banner `Previewing “Login v2” — 2h ago`; comments/presence disabled. |
| Diff summary | Added/removed/changed nodes, components, tokens, styles, prototype links; per-page counts; `who changed what` grouped by `HistoryEvent.actorId`. |
| Restore | **Restore** requires preview + confirmation; applies the inverse diff as new commands on the current revision (never rewrites history), creates a safety `Version` first, and records `history.rollback`. |
| Agent versions | Runs touching >25 nodes auto-create a `Version` with `agentRunId` (02 §13); visible in timeline and linked to the run summary. |
| Retention | Versions retained 90 days for free workspaces, 1 year for paid; audit log 1 year; legal hold overrides. |

Restore preserves current sharing/permissions; it does not restore deleted files, library links, or comments. Comments remain on their original coordinates and may detach if anchors vanished.

### 6.5 Activity log, notifications, agent mentions

- **Activity log** is append-only `AuditLogEntry` with kinds `COMMAND`, `PERMISSION`, `CONFIRMATION`, `GUARDRAIL_BLOCK`, `EXPORT`, `PUBLISH`, `CONNECTION`. Filter by actor/kind/date; export CSV (owner). It answers "why did this change?" and is not editable.
- **Inbox** groups: mentions, replies, resolves, library updates, agent run finished/failed/needs approval, access requests. Unread badge on the bell; `⌘⇧N` opens.
- **@agent** in a comment creates a run only if the agent is in `FileMeta.agentPolicy.allowedAgents` and the actor has comment permission. The run prompt includes the comment, thread context, page/frame, and selected node ids; confirmation shows scopes and budget.
- Agent run completion posts a system comment in the originating thread with summary + warnings (no mention loop).

### 6.6 Permissions & sharing

| Capability | Owner | Editor | Commenter | Viewer |
|---|---|---|---|---|
| View canvas/player | Yes | Yes | Yes | Yes |
| Comment / mention | Yes | Yes | Yes | No |
| Edit document | Yes | Yes | No | No |
| Manage pages/import/export | Yes | Yes | No | No (export if link allows) |
| Publish library / manage tokens | Yes | Yes with scope | No | No |
| Share / change roles | Yes | No | No | No |
| Delete file / transfer owner | Yes | No | No | No |

`FileMeta.sharing.visibility`: `PRIVATE` (invited only), `LINK` (anyone with link gets `linkRole`), `WORKSPACE` (workspace members get their workspace role). Guest links are `LINK` + `VIEW`/`COMMENT`; guests must enter a display name, cannot be mentioned persistently, and see no library admin surfaces. Password/expiry links are V2 (schema addition).

**Agent access.** `FileMeta.agentPolicy.allowedAgents` gates connection; `AgentSession.grantedScopes` gates operations; `Agent.scopes` is the ceiling. Agents never receive sharing/manage permissions. Revoking an agent stops new commands within 250ms and preserves its history/audit. Guest viewers can never trigger agents.

### 6.7 Offline and reconnect

- Editing works offline against an IndexedDB Yjs copy for up to 15 minutes or 1,000 queued commands (whichever first), then becomes read-only with a banner.
- Presence is local-only offline; remote cursors disappear after 30s.
- Reconnect: sync, replay queued commands through the gateway, then reconcile. Rejected commands (permission/deleted node) surface in **Review changes** with the reason.
- Conflict UX is CRDT merge plus the explicit review described in §6.3. No "choose a version" modal for ordinary merges; version restore is separate.
- Comments queue up to 50 and send in order; versions and publishes are blocked offline.

### 6.8 Collaboration acceptance criteria

1. Two editors changing the same fill converge to one value on both clients within 500ms p95; the loser sees a named toast.
2. Deleting a node another user is dragging rejects the drag with `NODE_DELETED`; the node is restorable for 30 days.
3. Presence expires after 30s; stale cursors never remain on canvas.
4. A node-anchored comment moves with its node across frames/pages; a canvas pin stays at coordinates.
5. Resolving/reopening updates all clients and the filter counts; mentions notify exactly once.
6. Version preview is read-only, restore creates a new revision and safety version, and the timeline shows the actor of every change.
7. Viewer cannot edit/comment/export beyond link role; agent scope denial is audited.
8. Offline edits merge on reconnect; rejected commands are listed with recovery, never silently dropped.

### 6.9 Agent-relevant affordances — collaboration

| Capability | Level | Constraint |
|---|---|---|
| Read comments/presence, reply when mentioned | `SAFE` | `COMMENT_WRITE`; no unsolicited threads on protected frames |
| Create comments/annotations during a run | `SAFE` | Max 20 per run; no mentions unless user asked |
| Create version/checkpoint | `SAFE` | `VERSION_CREATE`; auto versions already required before large runs |
| Resolve others' threads | `CONFIRM` | Only own threads automatically |
| Change sharing/permissions | `PROTECTED` | Human owner only |

---

## 7. File Structure, Import/Export (Feature Surface)

### 7.1 Multi-page files and file browser

| Surface | Behavior |
|---|---|
| Page tabs | Horizontal tabs with overflow menu; `+` creates `page.create`; double-click renames; drag reorders (`page.reorder`); duplicate copies the page forest (`page.duplicate`). |
| Page delete | Confirmation names node count; nodes are trashed (30 days) and the page is removed; recover through version history. Cannot delete the last page. |
| Page background | Solid color or none; applies to canvas and player; not exported unless scope includes background. |
| File browser | Grid/list, search by name/owner, sort by modified/created/name, duplicate, move between workspaces, archive, trash (30 days), restore. Folder hierarchy is V1. |
| Templates | Workspace-marked files; **Use template** duplicates the file (`file.createFromTemplate`), resets sharing to private, preserves tokens/components. |
| Lifecycle | Archive/trash are workspace metadata outside `KanvasFile`; they never mutate `FileMeta` schema. |

File commands (`file.create`, `file.duplicate`, `file.rename`, `file.move`, `file.archive`, `file.restore`, `file.delete`, `file.createFromTemplate`) are workspace operations, not `DesignCommand`s. Page commands are document commands.

### 7.2 Import

| Format | Support | Limits / rules |
|---|---|---|
| SVG | MVP | Sanitization and mapping in §4.3; editable vectors default; text outlined; max 10MB. |
| PNG/JPEG/WebP/AVIF | MVP | Placed as `IMAGE` or fill; max 50MB / 8192px; EXIF orientation applied; color profile converted to sRGB. |
| `.kanvas` | MVP | ZIP container per 02 §17; validated (`schema`, `graph`, `references`, `variables`, `limits`); imported as a **new file** with new `f_` id, internal ids preserved; invalid files rejected with a report. |
| Kanvas JSON | MVP | Same validators; must include manifest + at least one page; imported into current file as a new page or new file (user choice). |
| `.fig` | **Out of MVP** | Roadmap V2; UI states "Not supported yet" and links to SVG/JSON import. Never silently attempts a partial parse. |
| PDF | Out of scope | Not an import format. |

Import runs as an operation (`import.run`) that emits normal commands with `origin: 'IMPORT'`; it is cancellable until commit and produces one undo group. Errors list row/element, reason, and suggested fix. Import never overwrites an existing page/file without explicit **Replace** confirmation.

### 7.3 Export UI

**Entry points:** toolbar **Export**, right-click selection **Export…**, File menu **Export file**, Prototype player **Export frame**, Inspect tab **Export**.

| Option | Values / behavior |
|---|---|
| Scope | Selection, frame(s), page, file (all pages); multi-selection exports one file per root. |
| Formats | PNG, JPG, SVG, PDF, WebP. SVG only for vector-capable roots; PDF for frames/pages. |
| Scale | 0.5×–4× in 0.5 steps; default 1×; independent per format preset. |
| Suffix | `@1x`, `@2x`, custom suffix; sanitized, max 32 chars; preview filename. |
| Background | Transparent (PNG/WebP/SVG), include canvas/page background, or custom color. JPG/PDF force a background with tooltip. |
| Presets | 1×/2×/3×, iOS (`@1x/@2x/@3x`), Android (`mdpi…xxxhdpi`), Web (`1x/2x`); presets are named UI configs, not document schema. |
| Batch/ZIP | >1 output → ZIP named `{file}-{scope}-{date}.zip`; folder per page; manifest.json with filenames and scale. |
| PDF | One page per top-level frame, ordered by canvas position (x then y); page scope = one PDF; file scope = one PDF per page zipped. |
| Limits | Max 500 outputs per job; max 8192px per edge; max 200MB uncompressed; exceeding blocks with a message. |

**Progress/failure.** Job states `QUEUED → RENDERING → PACKAGING → READY → FAILED/CANCELLED`. Per-item progress and a cancel button; failure lists item, reason, and **Retry failed** without re-rendering successes. Temp artifacts expire after 24h. Export does not mutate the document; export settings on nodes use `exportSettings` and are saved normally. Detailed renderers and codegen live in `08-export-and-codegen.md`.

### 7.4 Handoff to code

Entry points: **Export → Code**, Inspect tab **Code**, and the library **Sync tokens** action. Options: framework `REACT_TAILWIND`, `HTML_CSS`, `TOKENS_CSS`, `TOKENS_JSON`; scope selection/frame/page; include assets; theme (default or explicit). The panel shows a file tree preview, estimated size, and dependency notes. Job states match §7.3 plus `VALIDATING`; validation failures are per-file with line numbers. Download is a ZIP; no direct repo write. Codegen behavior, framework mapping, and token emission are normative in `08-export-and-codegen.md`.

### 7.5 Import/export acceptance criteria

1. `.kanvas` import validates all five validators and rejects a corrupted checksum without partial pages.
2. SVG import produces editable vectors for paths/groups/gradients and outlines text with a visible warning.
3. Export 0.5×–4× produces exact pixel dimensions for PNG/WebP and a valid multi-page PDF ordered by canvas position.
4. Batch of 100 frames completes with a manifest and per-item errors; retry only failed items.
5. Transparent background is disabled for JPG/PDF with an explanatory tooltip.
6. Import/export never mutates design content except the explicit import commit; both are attributable and cancellable.

### 7.6 Agent-relevant affordances — files/import/export

| Capability | Level | Constraint |
|---|---|---|
| Create/rename/reorder pages, duplicate page | `SAFE` | `PAGE_MANAGE`; never delete a page without confirmation |
| Import asset/SVG/JSON | `CONFIRM` | Validation report shown; limits enforced |
| Export selection/frame/file | `SAFE` | `EXPORT_RUN`; no export of protected library content without rights |
| Delete/archive file, move workspace | `PROTECTED` | Human owner only |
| Generate code artifact | `SAFE` | `CODE_GEN`; read-only download, no repo write |

---

## 8. Cross-Cutting Error & Empty States

| Surface | State | UI | Recovery | Telemetry |
|---|---|---|---|---|
| Components | Dangling master | Amber `DANGLING` chip; snapshot renders | Restore link / relink library / detach | `component.dangling` |
| Components | Deleted library | Library card `Deleted`, link count | Re-link or detach all | `library.deleted` |
| Components | Variant mismatch | Default variant + warning | Pick valid combo | `variant.fallback` |
| Components | Empty variant master | Dashed preview | Fill/remove; publish blocked | `variant.empty` |
| Tokens | Alias cycle/unresolved | Magenta value, `Unresolved` | Repair/repoint | `token.unresolved` |
| Tokens | Delete in use | Blocking usage list | Replace/detach | `token.delete_blocked` |
| Tokens | Theme incomplete | Card `Incomplete` | Assign missing mode | `theme.incomplete` |
| Tokens | Import parse failure | Line/column report, no changes | Fix/re-import | `token.import_failed` |
| Assets | Missing asset | Checkerboard + badge | Relink/replace | `asset.missing` |
| Assets | Unsupported SVG | Import report | Rasterize/simplify | `asset.svg_rejected` |
| Assets | Font missing | Banner + fallback metrics | Replacement mapping | `font.missing` |
| Prototype | Broken link | Red connector + error | Select/fix destination | `prototype.broken` |
| Prototype | Auto-loop | Blocked at 10 cycles | Break loop | `prototype.loop_blocked` |
| Prototype | AI budget exceeded | Inline error, no mutation | Raise budget/retry later | `prototype.ai_blocked` |
| Collaboration | Offline | Banner + queued count | Auto-sync | `sync.offline` |
| Collaboration | Permission denied | Disabled control + reason | Request access | `permission.denied` |
| Files | Export failure | Per-item error + retry | Retry failed | `export.failed` |
| Files | Import invalid | Validator report | Fix file | `import.invalid` |
| Global | Node/file limit | Typed error with current/max | Split file/page | `limits.exceeded` |

Empty states always name the object, explain why it is empty, and offer one primary action. Errors never expose stack traces, never auto-delete dangling references, and always preserve user input/drafts.

---

## 9. Acceptance Criteria Summary & Performance Budgets

**Release gate.** Every section's acceptance criteria pass, plus:

1. A Figma user completes "create component → instance → override → variant set → token bind → prototype link → present → comment" unaided in ≤10 minutes.
2. Undo restores every operation in this document, including library update merge, token delete/rebind, version restore, and asset replace, as one user-visible action per gesture.
3. No dangling reference is silently deleted; all appear in a recovery surface.
4. All mutation paths are command-based and attributable; no UI action bypasses `HistoryEvent`.
5. Agent-originated changes are visible, confirmable where required, and reversible per run.
6. Keyboard-only paths exist for every control named here; axe reports zero critical issues on library, token, prototype, and comment panels.
7. Theme resolution is byte-identical across editor, player, PNG/SVG export, and token CSS export for the same theme.
8. A file with 40k nodes, 2k components, 5k tokens, and 200 flows opens, searches, and presents within the budgets below.

| Operation | Budget (p95) | Conditions |
|---|---|---|
| Instantiate 200-node master | ≤50ms | Warm cache, same page |
| Root swap / variant swap | ≤100ms | 200-node instance |
| Component browser search (5k components) | ≤200ms | Local + linked metadata cached |
| Token panel first paint (2k tokens) | ≤300ms | Virtualized |
| Theme switch (5k bound nodes) | ≤100ms | No layout change |
| Bind/unbind one property | ≤16ms | Single command |
| Token import (10k tokens) | ≤5s | Valid JSON |
| Asset ingest 12MP JPEG | ≤3s | 10Mbps upload, server encode |
| Asset grid scroll (500 assets) | 60fps | Thumbnails cached |
| SVG import 10MB | ≤5s | Editable vectors |
| Prototype navigation | ≤100ms | Destination page cached |
| Trigger-to-action latency | ≤50ms | Local runtime |
| Player animation | 60fps | 2k-node frame |
| Presence cursor update | ≤150ms | WAN, 20Hz |
| Comment optimistic render | ≤100ms | Local echo |
| Version restore (50k nodes) | ≤3s | Snapshot cached |
| Export 100 frames ZIP | ≤60s | 2× PNG, server render |
| Offline queue sync (1k commands) | ≤5s | Reconnect, no conflicts |

Budgets are measured on the reference hardware in `06-architecture.md` §9; exceeding a budget by >20% blocks release for that surface unless waived in writing.

---

## 10. Explicitly Deferred / Open Questions

| Item | Status | Reason / next step |
|---|---|---|
| `.fig` import | Deferred (V2) | Undocumented format; SVG/JSON import covers MVP (00 D-009). |
| Arbitrary children-as-slot inside instances | Deferred (schema) | 02 mirrors instance children; needs `InstanceNode` slot structure and migration. Use `INSTANCE_SWAP` slots (§2.7). |
| Conditional prototype logic | Deferred (V2) | `Interaction` has no condition field; needs schema addition. Model with variants/frames until then. |
| Per-frame theme pin | Deferred | `FrameNode`/`Page` have no theme field; needs schema. |
| Publishing `Theme` as a library item | Deferred | `Theme` has no `libraryId`; publish collections/modes instead. |
| `Variable` deprecation field | Open | MVP annotation is library metadata; propose `deprecatedAt` in schema v2. |
| Property type change | Deferred | Recreate semantics and override migration need a dedicated tool. |
| Child transform overrides inside instances | Deferred | Conflicts with deterministic auto layout; needs layout-safe override model. |
| Manual Smart Animate pairing | Deferred | 02 matching order is fixed; manual pairing needs an interaction field. |
| Variant-to-variant animated transition | Deferred | Use frame navigation + Smart Animate. |
| Password/expiry guest links | Deferred | `FileMeta.sharing` has no fields; schema/platform change. |
| Team asset library publish | Deferred (V1) | `Library.items` has no `assetIds`; per-asset `remoteLibraryId` only. |
| Runtime AI prototypes (`AI_GENERATE`, `AI_VARIANT`) | V2 | Cost, consent, safety; semantics specified in §5.13. |
| Token/style automatic conversion | Deferred (V1) | No lossless mapping for multi-paint styles. |
| Comments on resolved anchors after page deletion | Open | Keep `position` fallback; decide notification behavior. |
| Template provenance | Open | Workspace metadata now; consider `FileMeta.templateId`. |

---

## 11. New Identifiers Introduced

All names below are introduced by this document. Per 02's header rule, the doc owner MUST add them to the glossary in `README.md`; this file intentionally edits no other document.

**DesignCommands (document mutations).**

`component.create`, `component.update`, `component.delete`, `component.duplicate`, `component.protect`, `component.unprotect`, `componentSet.create`, `variant.add`, `variant.remove`, `variant.rename`, `variant.setDefault`, `component.instantiate`, `component.swap`, `component.detach`, `instance.relink`, `instance.setOverride`, `instance.resetOverrides`, `instance.resetOverrides`, `instance.setComponentProperty`, `componentProperty.create`, `componentProperty.update`, `componentProperty.delete`, `componentProperty.expose`, `library.publish`, `library.link`, `library.unlink`, `library.update`, `library.rollback`, `library.setPolicy`, `variable.create`, `variable.update`, `variable.update`, `variable.delete`, `variable.bind`, `variable.unbind`, `variable.rebind`, `variable.setValue`, `variable.update`, `variable.extract`, `collection.create`, `collection.update`, `collection.delete`, `variable.createMode`, `variable.updateMode`, `variable.deleteMode`, `collection.update`, `theme.create`, `theme.update`, `theme.duplicate`, `theme.delete`, `theme.setDefault`, `style.createStyle`, `style.updateStyle`, `style.updateStyle`, `style.deleteStyle`, `style.applyStyle`, `style.detachStyle`, `asset.upload`, `asset.register`, `asset.update`, `asset.update`, `asset.update`, `asset.delete`, `asset.replace`, `asset.insert`, `asset.update`, `image.setScaleMode`, `image.setCrop`, `image.setFilters`, `image.rotate`, `font.register`, `font.replace`, `prototype.link`, `prototype.removeLink`, `prototype.updateLink`, `prototype.reorderInteraction`, `prototype.setFlow`, `prototype.setFlow`, `prototype.deleteFlow`, `comment.create`, `comment.reply`, `comment.edit`, `comment.delete`, `comment.resolve`, `comment.reopen`, `comment.react`, `comment.move`, `history.createVersion`, `history.rollback`, `page.create`, `page.rename`, `page.reorder`, `page.duplicate`, `page.delete`.

**Operations (not document mutations).**

`file.create`, `file.duplicate`, `file.rename`, `file.move`, `file.archive`, `file.restore`, `file.delete`, `file.createFromTemplate`, `import.run`, `export.run`, `export.cancel`, `variable.deprecate`, `library.dismissUpdate`.

**Runtime, validation, and lint identifiers.**

`PrototypeRuntimeState` (`SETTLED`, `ARMED`, `FIRING`, `TRANSITIONING`, `BLOCKED`); lint IDs `token/name-format`, `token/category-mismatch`, `token/primitive-alias`, `token/component-alias-primitive`, `token/component-alias-chain`, `token/type-mismatch`, `token/scope-mismatch`, `token/alias-cycle`, `token/alias-depth`, `token/semantic-literal`, `token/component-unscoped`, `token/orphan`, `token/deprecated-in-use`, `token/hardcoded-value`, `token/contrast`, `token/ambiguous-role`; validation IDs `override/ambiguous-path`, `override/path-depth`, `component/property-conflict`, `instance/variant-mismatch`, `variant/empty-master`, `library/breaking-change`, `asset/missing-license`, `prototype/broken-link`, `prototype/unreachable`, `prototype/dead-end`, `prototype/auto-loop`, `prototype/loop`, `prototype/missing-destination`, `prototype/invalid-overlay`, `prototype/invalid-variable`, `prototype/invalid-variant`, `prototype/duplicate-key`, `prototype/terminal-actions`, `prototype/empty-flow`.

**UI-only concepts (not persisted, not command names).** `previewThemeId`, `orphanedOverrides`, `PrototypeRuntimeState` transitions, `Review changes`, `ExportPreset`, `LibraryPublish` (publish artifact referenced by 02 §11.2), `DANGLING`/`DETACHED` badges.

