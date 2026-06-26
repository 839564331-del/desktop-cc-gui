## Context

The current codebase already has meaningful quality infrastructure: strict TypeScript, Vitest coverage, Rust tests, runtime contract scripts, doctor checks, OpenSpec changes, and large-file governance. The problem is therefore not lack of discipline; the problem is that several high-churn areas still concentrate too much orchestration in large files.

Observed hotspots include:

- `src/services/tauri.ts` as a mixed-domain frontend/backend bridge with many exports.
- `src/features/threads/hooks/useThreads.ts`, `useThreadsReducer.ts`, `useThreadActions.ts`, and `useThreadMessaging.ts` as coupled chat/runtime/history/message orchestration surfaces.
- `src-tauri/src/codex/mod.rs`, `src-tauri/src/backend/app_server.rs`, `src-tauri/src/runtime/mod.rs`, and `src-tauri/src/computer_use/mod.rs` as backend runtime-critical modules near large-file warning thresholds.
- `SettingsView.tsx`, `Composer.tsx`, Git History panel files, and several CSS files as high-churn UI surfaces near governance thresholds.

This design treats complexity reduction as a product-safety change. It must preserve behavior first and only reduce structure debt second.

## Goals / Non-Goals

**Goals:**

- Establish clear module boundaries for core bridge, thread, runtime, and UI orchestration code.
- Keep existing behavior stable while reducing file size, import coupling, and review scope.
- Convert hidden cross-layer assumptions into focused tests and runtime contract checks.
- Make each extraction independently reviewable and reversible.
- Finish with full regression validation, not only focused tests.

**Non-Goals:**

- No user-facing feature changes.
- No product redesign.
- No new dependency.
- No storage migration.
- No Tauri command contract change unless split into another OpenSpec change.
- No Project Memory V2 implementation.

## Decisions

### Decision 1: Use incremental extraction, not rewrite

Adopt a strangler-style extraction: keep existing public entry points stable, move implementation behind them, then migrate internal callers only when safe.

Alternatives considered:

- Big-bang rewrite: rejected because review and rollback become unsafe.
- Opportunistic cleanup: rejected because it does not create a reliable regression boundary.

Rationale: behavior preservation is the primary requirement. Incremental extraction lets each batch prove equivalence before moving on.

### Decision 2: Keep compatibility exports during service bridge split

`src/services/tauri.ts` should continue exporting existing functions while domain-specific modules are introduced under `src/services/tauri/**`. Initial extraction should move implementation and types to domain files, then re-export from the legacy bridge.

Alternatives considered:

- Update all import sites immediately: rejected because it increases churn and obscures whether behavior changed.
- Leave `tauri.ts` untouched: rejected because it keeps the largest cross-layer risk intact.

Rationale: compatibility exports provide a stable migration shell while allowing domain modules to become the new ownership boundary.

### Decision 3: Extract pure functions before moving hook orchestration

For `threads`, first extract deterministic logic such as event normalization, turn snapshot resolution, queue settlement classification, reducer helpers, and history hydration mapping. Only after pure functions are covered should hook-level orchestration move.

Alternatives considered:

- Split hooks by file sections first: rejected because mechanical cuts can preserve poor boundaries.
- Rewrite the thread state machine: rejected because it creates behavior risk outside this change.

Rationale: pure functions are easier to test and create a stable seam for later hook simplification.

### Decision 4: Backend module extraction must preserve command registry behavior

Rust module extraction must keep `#[tauri::command]` names, registrations, payload shapes, and error semantics stable. `command_registry.rs` should change only to point to equivalent module paths or continue using existing re-exported paths.

Alternatives considered:

- Rename commands while refactoring: rejected because that is a cross-layer behavior change.
- Split command behavior and diagnostics together: rejected unless tests already prove equivalence.

Rationale: Tauri command names are runtime contracts, not internal implementation details.

### Decision 5: Full regression is a completion gate

Focused tests are required for each batch, but they are insufficient for completion. The final gate must include lint, typecheck, all frontend tests, runtime contract checks, strict doctor, large-file governance, full Rust tests, and manual smoke evidence.

Alternatives considered:

- Focused-only validation: rejected because this change intentionally touches core seams.
- Full regression after every tiny task: rejected because it is too slow and discourages small commits.

Rationale: use focused validation for fast local confidence, full regression for release confidence.

## Migration Plan

### Phase 0: Baseline and boundaries

- Record current large-file watchlist and affected modules.
- Define allowed write scopes for each batch.
- Identify focused tests for each touched domain.
- Confirm no behavior changes are included.

### Phase 1: Tauri bridge facade extraction

- Move domain implementations from `src/services/tauri.ts` into `src/services/tauri/<domain>.ts`.
- Keep old exports stable.
- Add or update service contract tests.
- Run typecheck and runtime contract checks.

### Phase 2: Threads main path extraction

- Extract pure adapters for event normalization, history hydration, queue settlement, and reducer helpers.
- Preserve hook public API.
- Add focused tests for extracted adapters and existing regression suites.
- Avoid Project Memory V2 behavior changes.

### Phase 3: Backend runtime-critical extraction

- Split Codex, app-server, runtime, and computer-use backend modules by responsibility.
- Preserve command registry and payload behavior.
- Add targeted Rust tests around launch, probe, diagnostics, pool, and recovery seams.

### Phase 4: UI and CSS debt reduction

- Split Settings, Composer, and Git History orchestration surfaces into presentational components and interaction hooks.
- Split CSS by existing feature sections without visual redesign.
- Split oversized tests by behavior group.

### Phase 5: Full regression and evidence

- Run the full automated gate.
- Execute manual smoke matrix.
- Record verification evidence in the change directory before archive.

## Risks / Trade-offs

- Regression hidden by mechanical move -> Mitigation: write focused tests before moving logic and run full regression at the end.
- Import churn creates noisy diffs -> Mitigation: keep compatibility exports and migrate callers gradually.
- Backend command path changes accidentally break Tauri registration -> Mitigation: preserve re-exported command paths or update `command_registry.rs` with contract checks.
- CSS split causes selector ordering regressions -> Mitigation: preserve import order and do no visual redesign in this change.
- Long-running full regression slows progress -> Mitigation: run focused gates per batch and full gate at phase completion.
- Scope creep into feature work -> Mitigation: any behavior change must get a separate OpenSpec change.

## Rollback Strategy

- Each phase should be committed separately.
- If a batch regresses behavior, revert that batch only and keep earlier validated extractions.
- Keep legacy facade exports until all call sites are migrated and validated.
- Avoid storage or command contract changes so rollback does not require data migration.

## Open Questions

- Which exact service domain should be extracted first from `src/services/tauri.ts`: Git, workspace, threads, or runtime?
- Should large-file warning reduction be measured as total warning count, P0 warning count, or touched-file delta?
- Should manual smoke evidence be stored as `verification.md` in this OpenSpec change or in a Trellis task journal as well?
