## Why

Codex 自动压缩本身已经可以在阈值命中后触发，但前端当前把“历史上发生过压缩”与“当前压缩状态”混在了一起。结果是 tooltip 和幕布可能同时展示 `140%` 这类旧用量快照与“已压缩”文案，用户会误判为自动压缩未生效。

这个问题已经开始影响用户对 Codex 自动压缩能力的信任，因此需要把前端状态语义和可见 copy 一次性收口成可解释、可验证的行为。

## 目标与边界

### 目标

- 明确区分“压缩生命周期状态”和“最后一次背景信息用量快照”。
- 让 Codex 自动触发压缩时，幕布上的开始/完成文案始终与真实生命周期一致。
- 在 compaction 完成但 usage snapshot 尚未刷新时，tooltip 给出专业、准确、不误导的提示。
- 保持手动压缩与自动压缩共用同一条生命周期链路，不分叉出第二套 UI 状态机。

### 边界

- 仅修正前端状态建模、事件衔接与 user-visible copy。
- 不调整 Codex 自动压缩阈值、cooldown、in-flight 防重入等 backend 触发逻辑。
- 不新增独立的 compaction panel 或新的 settings 入口。

## 非目标

- 不重做 Composer dual-view 的整体视觉设计。
- 不引入新的 backend event 类型。
- 不扩展 Claude/OpenCode/Gemini 的 compaction 语义。

## What Changes

- 修正 Codex compaction lifecycle 在前端的状态来源，避免用历史消息永久污染当前 tooltip 状态。
- 为 `thread/compacted` completion 缺少 source flags 的情况保留 source continuity，确保自动触发压缩的幕布文案仍然正确。
- 调整 Codex dual-view tooltip 的状态文案，使其能明确表达“压缩已完成，但背景信息用量正在等待刷新”等过渡状态。
- 补齐相关测试，覆盖 auto/manual compaction 的开始、完成、历史恢复与 usage 未刷新的边界条件。

## 技术方案

### 方案 A：只改 copy，不改状态来源

- 优点：改动小、交付快。
- 缺点：核心误导仍然存在，旧 usage snapshot 与历史 compaction message 仍会把 UI 推向错误状态。

### 方案 B：收口 lifecycle state，再在其上改 copy

- 做法：把当前 compaction 状态从“历史消息是否存在”改为“线程状态 + 最近一次 compaction lifecycle metadata”，并在 usage snapshot 未刷新时显式展示 sync-pending 提示。
- 优点：语义稳定，自动/手动压缩都能解释清楚，测试边界明确。
- 成本：需要调整 reducer / event handler / dual-view derive logic，并补若干回归测试。

取舍：采用方案 B。这个问题不是单纯翻译问题，而是状态语义问题；只改文案无法消除误导。

## Capabilities

### New Capabilities

（无）

### Modified Capabilities

- `codex-context-auto-compaction`：更新自动/手动压缩在可见消息面板上的生命周期语义，保证 completion 缺少 source flags 时仍能维持正确显示。
- `composer-context-dual-view`：更新 Codex dual-view tooltip 的状态语义，区分 lifecycle completion 与 usage snapshot 尚未刷新的过渡状态。

## Impact

- Frontend
  - `src/features/composer/components/Composer.tsx`
  - `src/features/composer/components/ChatInputBox/ContextBar.tsx`
  - `src/features/composer/components/ChatInputBox/types.ts`
  - `src/features/threads/hooks/useThreadsReducer.ts`
  - `src/features/threads/hooks/useThreadTurnEvents.ts`
- i18n
  - `src/i18n/locales/zh.part2.ts`
  - `src/i18n/locales/en.part2.ts`
- Tests
  - `src/features/composer/components/Composer.context-dual-view.test.tsx`
  - `src/features/composer/components/ChatInputBox/ContextBar.test.tsx`
  - `src/features/threads/hooks/useThreadTurnEvents.test.tsx`
  - `src/features/threads/hooks/useThreadsReducer.history-restore.test.ts`

## 验收标准

- 自动压缩触发后，幕布必须显示与真实 lifecycle 一致的开始/完成文案，不得因 completion 缺少 source flags 而回退成误导性状态。
- Tooltip 在 compaction 完成但 usage snapshot 尚未刷新时，必须展示明确的过渡提示，而不是直接用“已压缩”掩盖旧用量快照。
- 历史消息恢复后，tooltip 不得仅因存在旧 compaction message 就永久停留在 completed 状态。
- 相关前端用例必须覆盖 auto/manual lifecycle、payload-less completion、history restore 和 stale usage snapshot。
