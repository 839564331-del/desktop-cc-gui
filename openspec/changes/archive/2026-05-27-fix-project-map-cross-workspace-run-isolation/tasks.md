## 1. Frontend Run Ownership

- [x] 1.1 P0: Capture Project Map generation worker context at run start (`workspaceId`, `storageKey`, storage location, `runId`, worker dataset); output: hook uses worker-local dataset updates; validation: focused hook regression.
- [x] 1.2 P0: Remove progress/completion/failure dependence on mutable active `datasetRef.current`; output: stale run callbacks cannot mutate another workspace; validation: workspace switch regression.
- [x] 1.3 P0: Add frontend persistence ownership preflight; output: mismatched dataset manifest rejects before IPC; validation: persistence unit test.

## 2. Backend Storage Boundary

- [x] 2.1 P0: Validate incoming `manifest.json.storageKey` in `project_map_write_snapshot`; output: backend rejects ownership mismatch atomically before file writes; validation: Rust unit test.
- [x] 2.2 P1: Ensure read-side dataset assembly rejects/quarantines manifest/storage-key mismatch; output: UI does not render polluted persisted snapshots; validation: persistence unit test.

## 3. Verification

- [x] 3.1 P0: Add regression tests for in-flight workspace switch and storage mismatch rejection; output: tests fail before fix and pass after fix.
- [x] 3.2 P0: Run `openspec validate --all --strict --no-interactive`, focused Vitest, Rust Project Map tests, and `npm run typecheck`; output: validation result recorded in final response.
