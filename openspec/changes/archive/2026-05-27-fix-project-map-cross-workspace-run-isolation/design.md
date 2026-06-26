## Context

Project Map data is persisted per workspace-derived storage key under the local project-map store. The current generation hook starts async workers with a `workspaceId`, but progress/completion/failure paths can later read `datasetRef.current`, which follows the currently active workspace rather than the worker's original workspace. The backend write command also trusts incoming files and does not validate that `manifest.json` belongs to the target workspace storage key.

This creates a two-layer failure mode: stale frontend workers can mix datasets, and the backend accepts mismatched snapshots. Read paths can then load polluted data as valid because the response storage key is derived from the requested workspace even when the persisted manifest says otherwise.

## Goals / Non-Goals

**Goals:**

- Bind each Project Map generation run to an immutable ownership context: `workspaceId`, derived `storageKey`, storage location, `runId`, and worker-local dataset.
- Ensure stale async run callbacks cannot mutate the active workspace dataset after a workspace switch.
- Reject snapshot writes when persisted manifest ownership does not match the target workspace.
- Reject/quarantine mismatched persisted snapshots on read.
- Cover the failure chain with regression tests.

**Non-Goals:**

- No cleanup or migration of existing local project-map data.
- No change to AI prompt semantics, graph layout, node scoring, or candidate workflow.
- No new storage schema version.

## Decisions

### Decision 1: Worker-local dataset ownership

Generation callbacks will mutate a run-local dataset reference captured at run start instead of `datasetRef.current`. UI state writes remain conditional on the active workspace/storage key still matching the worker context.

Alternative considered: only discard completion results when `workspaceIdRef.current !== workspaceId`. This is insufficient because progress/failure writes still call persistence and because a switched workspace can share mutable state paths through `datasetRef.current`.

### Decision 2: Frontend persistence preflight

Before any Project Map write, frontend persistence will verify that `dataset.manifest.storageKey` matches the expected storage key for the workspace being written. Worker writes will use the run's storage location rather than the current active read location.

Alternative considered: rely only on backend validation. Backend validation is necessary but not enough for developer ergonomics; frontend preflight keeps the bad call close to the originating hook and avoids unnecessary IPC.

### Decision 3: Backend snapshot ownership gate

`project_map_write_snapshot` will parse incoming `manifest.json` when present and reject writes if `manifest.storageKey` differs from the storage key derived from the `workspace_id` argument.

Alternative considered: silently rewrite manifest ownership to the requested workspace. That would hide cross-workspace corruption and can launder foreign evidence into the wrong dataset. Rejecting is safer and easier to diagnose.

### Decision 4: Read-side mismatch quarantine

When loading persisted Project Map files, the frontend will treat a manifest/storage-key mismatch as invalid data and avoid rendering it as trusted Project Map content. The UI can still fall back to an empty dataset and an error/debug signal rather than displaying polluted nodes.

Alternative considered: auto-prune suspicious nodes based on source paths or run ids. That is brittle and can delete good data; user-managed cleanup is outside this change.

## Risks / Trade-offs

- Existing polluted snapshots will stop loading or keep showing an empty/quarantined state. → This is intentional; the user will delete polluted local data manually.
- Some legacy snapshots without manifest ownership may still need compatibility. → Validate only when manifest has a non-empty `storageKey`, and ensure newly written snapshots always include it.
- Additional guards may surface errors in tests that previously passed with invalid fixtures. → Update fixtures to include correct storage ownership.

## Migration Plan

1. Ship frontend worker isolation and persistence preflight.
2. Ship backend write rejection for mismatched manifest ownership.
3. Ship read-side mismatch quarantine.
4. User manually deletes known polluted local directories and regenerates maps.

Rollback is straightforward: revert this change set. No persisted schema migration is introduced.

## Open Questions

- None for implementation. Existing polluted data cleanup remains explicitly manual per user instruction.
