## ADDED Requirements

### Requirement: Behavior-Preserving Core Extraction

The system SHALL allow core module extraction only when existing user-visible behavior and public runtime contracts remain unchanged.

#### Scenario: Frontend service bridge extraction preserves callers

- **WHEN** functionality is moved out of `src/services/tauri.ts` into domain-specific service modules
- **THEN** existing frontend imports and exports MUST continue to work until callers are intentionally migrated
- **AND** Tauri command names, payload field names, and response shapes MUST remain unchanged

#### Scenario: Backend module extraction preserves command contracts

- **WHEN** Rust backend code is moved between modules
- **THEN** registered Tauri command names MUST remain available through `src-tauri/src/command_registry.rs`
- **AND** successful responses and error propagation semantics MUST remain equivalent for existing callers

#### Scenario: UI extraction preserves behavior

- **WHEN** Settings, Composer, Git History, Threads, or related CSS files are split
- **THEN** visible UI behavior, persisted state behavior, keyboard interactions, and i18n keys MUST remain unchanged unless another OpenSpec change authorizes a behavior change

### Requirement: Incremental Regression Evidence

The system SHALL require focused validation for each core extraction batch and full regression validation before completion.

#### Scenario: Focused validation after each batch

- **WHEN** a batch extracts code from a core frontend or backend surface
- **THEN** the implementer MUST run focused tests or contract checks covering the touched behavior
- **AND** failures MUST be fixed before continuing to the next batch

#### Scenario: Full regression gate before completion

- **WHEN** the change is marked complete
- **THEN** the final verification evidence MUST include `openspec validate reduce-core-complexity-preserve-behavior --strict`
- **AND** it MUST include `npm run lint`, `npm run typecheck`, `npm run test`, `npm run check:runtime-contracts`, `npm run doctor:strict`, `npm run check:large-files:near-threshold`, and `cargo test --manifest-path src-tauri/Cargo.toml`
- **AND** any skipped command MUST be documented with a concrete reason and residual risk

#### Scenario: Manual smoke matrix before completion

- **WHEN** automated full regression has passed or documented blockers are accepted
- **THEN** the implementer MUST record manual smoke evidence for app launch, workspace selection, Codex chat, thread history, settings persistence, file preview, and Git status/diff/history flows
- **AND** any unavailable engine-specific smoke path MUST be documented with environment constraints

### Requirement: Large-File Governance During Core Refactor

The system SHALL prevent core complexity refactors from increasing large-file debt in touched areas.

#### Scenario: Touched file approaches large-file threshold

- **WHEN** a touched source, style, or test file is near a configured large-file warning threshold
- **THEN** the implementation MUST either reduce its size or document why the file remains on the watchlist
- **AND** the implementation MUST run the configured large-file check before completion

#### Scenario: New module grows beyond intended boundary

- **WHEN** extracted code creates a new module that approaches the relevant warning threshold
- **THEN** the extraction MUST be revisited before completion
- **AND** the new module MUST be split by responsibility rather than becoming a replacement hub
