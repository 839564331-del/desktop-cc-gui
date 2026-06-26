## Why

mossx has reached the point where product behavior is protected by substantial tests and runtime contracts, but several high-churn core files still act as orchestration hubs. The current risk is not missing functionality; the risk is that future fixes to `threads`, `tauri` bridge, Codex/runtime launch, Settings, Composer, and Git History must repeatedly touch oversized files with large regression radius.

This change creates a behavior-preserving complexity governance plan: reduce core-file coupling, keep public behavior stable, and require full regression validation before the work can be considered complete.

## 目标与边界

### 目标

- Reduce the complexity of high-risk core surfaces without changing user-visible behavior.
- Split large orchestration files into domain modules, adapters, pure functions, and thin facades.
- Preserve existing Tauri command names, frontend service exports, runtime semantics, settings persistence, and UI behavior unless a later OpenSpec change explicitly changes behavior.
- Make regression safety explicit through focused tests, runtime contract checks, large-file governance, full frontend validation, and backend validation.
- Keep implementation incremental so each batch can be reviewed, tested, and rolled back independently.

### 边界

- The change covers refactoring and governance only.
- The change may move code, extract modules, add tests, and add regression documentation.
- The change may update code-level Trellis specs if implementation creates reusable contracts.
- The change must not introduce new persisted settings fields, new product behavior, new Tauri commands, or new external dependencies unless a later task explicitly creates a separate OpenSpec change for that behavior.

## 非目标

- Do not redesign the product UI or change visual language.
- Do not change Codex, Claude, Gemini, OpenCode, Git, workspace, thread, or project-memory behavior.
- Do not migrate Project Memory V2 in this change; that remains under `project-memory-refactor`.
- Do not change runtime launch precedence, authorization behavior, polling semantics, notification semantics, or file storage formats.
- Do not use this change to bundle unrelated bug fixes unless the fix is required to preserve behavior during extraction and is documented in tasks.

## What Changes

- Create a refactoring contract for behavior-preserving core complexity reduction.
- Split the frontend Tauri service bridge from a large mixed-domain facade into domain-specific service modules while keeping compatibility exports stable.
- Split the `threads` frontend main path into clearer units for event normalization, history hydration, queue settlement, memory-capture boundaries, and reducer slices.
- Split backend Codex/runtime/app-server modules along launch, diagnostics, command, pool, recovery, and model boundaries without changing command behavior.
- Split high-churn UI orchestration surfaces in Settings, Composer, and Git History into presentational components and interaction hooks.
- Reduce large CSS and test-file debt through mechanical moves and behavior-preserving grouping.
- Require a final full regression gate before completion:
  - `openspec validate reduce-core-complexity-preserve-behavior --strict`
  - `npm run lint`
  - `npm run typecheck`
  - `npm run test`
  - `npm run check:runtime-contracts`
  - `npm run doctor:strict`
  - `npm run check:large-files:near-threshold`
  - `cargo test --manifest-path src-tauri/Cargo.toml`
  - Manual smoke matrix for core flows.

## 技术方案对比与取舍

| Option | Description | Benefits | Risks / Costs | Decision |
|---|---|---|---|---|
| A | Big-bang rewrite of core surfaces | Can produce clean final structure quickly on paper | Very high regression risk; hard to review; hard to roll back; violates current stability needs | Reject |
| B | Opportunistic cleanup only when touching feature work | Low immediate cost | Complexity keeps growing; cleanup remains fragmented; no guarantee of full regression validation | Reject |
| C | Incremental behavior-preserving extraction with contract tests and full regression gate | Reviewable, reversible, aligned with existing OpenSpec/Trellis governance; protects behavior | Takes more coordination and repeated validation | Adopt |
| D | Freeze feature work until all complexity is removed | Maximizes focus | Too disruptive; blocks urgent bug fixes and active OpenSpec changes | Reject |

## Capabilities

### New Capabilities

- `core-complexity-governance`: Defines the behavior-preserving refactor contract, module extraction boundaries, compatibility expectations, and regression gate for core complexity reduction.

### Modified Capabilities

- None. This change is intentionally additive and does not modify existing product behavior requirements.

## Impact

- Frontend:
  - `src/services/tauri.ts`
  - `src/services/tauri/**`
  - `src/features/threads/**`
  - `src/features/settings/**`
  - `src/features/composer/**`
  - `src/features/git-history/**`
  - `src/styles/**`
- Backend:
  - `src-tauri/src/codex/**`
  - `src-tauri/src/backend/**`
  - `src-tauri/src/runtime/**`
  - `src-tauri/src/computer_use/**`
  - `src-tauri/src/command_registry.rs`
- Tests and governance:
  - Focused Vitest suites for touched frontend slices
  - Rust focused and full cargo tests for touched backend modules
  - Runtime contract checks
  - Large-file governance scripts and baselines
  - Trellis code-level specs when reusable implementation contracts emerge

## 验收标准

- User-visible behavior remains unchanged across core chat, runtime, workspace, settings, file, Git, and project navigation flows.
- Public frontend service exports remain compatible until all callers are migrated intentionally.
- Tauri command names, payload shapes, and response shapes remain unchanged unless a separate OpenSpec change explicitly approves a contract change.
- Large-file warning count is reduced or no new warnings are introduced for the touched files.
- Full regression gate passes before the change is marked complete.
- Manual smoke evidence is recorded for at least:
  - app launch and workspace selection
  - Codex session create/send/resume/interruption
  - Claude/Gemini/OpenCode basic send path where locally available
  - thread history load and realtime append
  - Settings read/write persistence
  - file tree and file preview
  - Git status/diff/history basic flows
