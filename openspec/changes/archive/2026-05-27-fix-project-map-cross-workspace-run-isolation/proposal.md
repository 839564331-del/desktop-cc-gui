## Why

Project Map generation can currently mix evidence and persisted nodes across two concurrently inspected workspaces. This is a P0 data isolation defect because one workspace can receive another workspace's knowledge map content while the UI still presents it as trusted project knowledge.

## 目标与边界

- Ensure every Project Map async run is owned by one immutable workspace/storage key context from start to finish.
- Reject persisted Project Map snapshots whose manifest ownership does not match the target workspace storage key.
- Quarantine mismatched snapshots on read instead of rendering cross-workspace data as valid knowledge.
- Do not repair or migrate already polluted local data in this change.

## 非目标

- No destructive cleanup of `~/.ccgui/project-map/*`.
- No redesign of Project Map generation prompts, graph layout, or evidence ranking.
- No change to normal incremental merge semantics inside the same workspace.

## What Changes

- Frontend Project Map run orchestration will stop reading mutable global active dataset state when an older run receives progress, completion, or failure callbacks.
- Project Map persistence will enforce manifest `storageKey` ownership before writing a snapshot.
- Tauri Project Map snapshot writes will parse `manifest.json` when present and reject ownership mismatches at the backend boundary.
- Project Map reads will treat manifest/storage-key mismatch as invalid persisted data and return an empty/quarantined dataset path to the UI.
- Regression tests will cover workspace switching while generation is in flight.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `project-map-incremental-generation`: Project Map generation and persistence must preserve workspace/storage-key isolation for async runs, writes, and reads.

## 技术方案选项

| 选项 | 做法 | 取舍 |
| --- | --- | --- |
| A | 仅在 frontend completion callback 判断当前 workspace 是否仍一致 | 能挡住 UI 覆盖，但挡不住旧 worker 写错目录，也挡不住未来其它调用方污染数据 |
| B | frontend worker 持有 immutable ownership context，并在 frontend persistence + backend write + read-side 三层做 storageKey gate | 覆盖完整故障链路，成本适中，是本次采用方案 |

采用 B。原因是这类 bug 的本质不是单点 callback stale，而是缺少跨层 ownership contract；必须在发起方、持久化边界、读取边界都显式验证。

## 验收标准

- 当 workspace A 的 Project Map 生成未完成时切换到 workspace B，A 的后续进度/完成/失败只能写入 A 的 storage key，不能污染 B 的 dataset 或 UI state。
- 写入 snapshot 时，若 `manifest.storageKey` 与目标 workspace 派生 storage key 不一致，backend MUST reject。
- 读取 snapshot 时，若 persisted manifest ownership 与 response storage key 不一致，frontend MUST NOT render that snapshot as valid Project Map data。
- 现有同 workspace incremental generation behavior 保持不变。

## Impact

- Frontend: `useProjectMapDataset`, Project Map persistence/service tests.
- Backend: `src-tauri/src/project_map.rs` snapshot write validation and Rust tests.
- Storage: no schema migration; invalid snapshots are rejected/quarantined by ownership checks.
- Dependencies: no new dependency.
