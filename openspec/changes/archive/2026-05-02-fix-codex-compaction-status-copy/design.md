## Context

当前 Codex dual-view summary 同时承担两类信息：

1. 压缩生命周期状态：是否正在压缩、是否刚刚完成压缩。
2. 背景信息窗口快照：最后一次 `thread/tokenUsage/updated` 或同类 usage 事件提供的占用率。

现状的问题在于，这两类信息来自不同时间轴，但前端把它们糅成了一个 `compacted` 语义：

- `Composer` 会从历史会话消息中查找 `context-compacted-*`，把它当成当前 completed 状态。
- reducer 还会刻意保留本地 Codex compaction message 穿过 history reconcile。
- `thread/compacted` completion 不一定带 `auto/manual` source flags，导致幕布 copy 在自动触发压缩时只能靠不稳定的推断。

这使得 UI 很容易出现“压缩已完成，但 usage 仍显示 140%”或“明明是老消息，tooltip 还一直显示已压缩”的误导状态。

## Goals / Non-Goals

**Goals:**

- 用显式 lifecycle metadata 表达“当前正在压缩 / 刚刚完成压缩 / 已回到普通状态”。
- 将 tooltip 状态 derive 从“历史消息存在性”切换到“thread status + lifecycle metadata + usage freshness”。
- 在 completion 缺少 `auto/manual` flags 时，延续 start 阶段已知的 source classification，保证自动压缩文案不漂移。
- 保留最后一次 usage snapshot 的可见性，但在其尚未刷新时给出准确解释。

**Non-Goals:**

- 不修改 Codex runtime 发出的 event name。
- 不新增 backend storage 字段。
- 不把手动压缩与自动压缩拆成两条不同 UI 组件路径。

## Decisions

### Decision 1: 在 frontend thread status 中补 lifecycle metadata，而不是继续扫描历史消息

- 方案 A：继续读取 `items[]` 中的 compaction message，并在 derive 时做更多过滤。
  - 问题：历史消息天然是“曾经发生过”，不是“当前状态”；再多过滤都容易继续混淆时间语义。
- 方案 B：在线程状态里记录最近一次 compaction source / completion freshness，`items[]` 只负责可见消息 surface。
  - 优点：状态源单一，tooltip 与幕布职责清晰。

取舍：采用方案 B。

### Decision 2: completion 缺少 source flags 时，沿用 in-flight source continuity

- 方案 A：要求 backend 为所有 `thread/compacted` 事件补齐 source flags。
  - 优点：协议显式。
  - 缺点：这次问题可在前端解决；额外改 backend contract 成本更高。
- 方案 B：前端在 `thread/compacting` 时记录 source，`thread/compacted` 缺 flags 时延续最近一次 in-flight source。
  - 优点：无协议变更，足以修复自动压缩显示正确性。

取舍：采用方案 B。

### Decision 3: tooltip completed 文案必须区分“压缩成功”与“usage 等待刷新”

- 方案 A：只显示“上下文已压缩”。
  - 问题：当最后一次 usage snapshot 仍高于阈值时，用户会直接理解为压缩没有生效。
- 方案 B：当 lifecycle completed 但 usage snapshot 仍未回落时，显示“压缩已完成，背景信息用量待刷新”一类 copy。
  - 优点：既不否认压缩成功，也不掩盖 snapshot 尚未刷新。

取舍：采用方案 B。

## Risks / Trade-offs

- [Risk] 新增 lifecycle metadata 后，Codex 与非 Codex 线程的 reducer 路径出现状态分叉
  → Mitigation：将 metadata 设计为可选字段，仅在 Codex compaction 路径写入；非 Codex 线程保持原行为。

- [Risk] tooltip 过渡文案过长，窄宽度下影响可读性
  → Mitigation：copy 保持单句、避免技术黑话，并用现有 tooltip 布局承载，不扩张结构。

- [Risk] completion 后迟迟收不到新 usage snapshot，sync-pending 状态持续较长
  → Mitigation：将该状态定义为 truth-preserving fallback；它比误导性“已压缩 + 140%”更可接受。

## Migration Plan

1. 先更新 OpenSpec delta，锁定 lifecycle 与 tooltip 语义。
2. 在 frontend thread status / compaction derive 层补 lifecycle metadata。
3. 调整 ContextBar / Composer dual-view derive 与 i18n copy。
4. 补 focused tests，覆盖 auto/manual、history restore、payload-less completion、stale usage snapshot。
5. 跑 `openspec validate --strict`、focused Vitest、`npm run lint`、`npm run typecheck`。

回滚策略：

- 若新 lifecycle metadata 引入回归，可回退到旧 derive 逻辑，但会重新暴露当前误导性提示问题。
- 本次不涉及 backend 协议改动，因此回滚范围局限于 frontend 与 i18n。

## Open Questions

- sync-pending 文案是否需要同时用于 tooltip 与幕布 completed copy，还是仅用于 tooltip 状态区。
- 是否要在后续迭代中为 backend completion 事件补 source flags，进一步减少前端推断。
