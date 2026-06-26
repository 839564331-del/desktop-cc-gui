## 1. Lifecycle State Modeling

- [x] 1.1 在 frontend thread status 中增加 Codex compaction lifecycle metadata（source / completion freshness），输入为现有 compaction events，输出为可供 tooltip 与幕布共用的显式状态；验证：`useThreadsReducer` / `useThreadTurnEvents` focused tests。
- [x] 1.2 调整 `thread/compacting` 与 `thread/compacted` 的事件衔接，确保 completion 缺少 source flags 时仍能延续自动/手动分类；验证：`useThreadTurnEvents.test.tsx` 回归。

## 2. Tooltip And Canvas Semantics

- [x] 2.1 重构 Codex dual-view 状态 derive，禁止仅凭 preserved historical compaction message 把当前 tooltip 判成 completed；验证：`Composer.context-dual-view.test.tsx`。
- [x] 2.2 更新 tooltip copy 与状态文案，在 compaction 完成但 usage snapshot 未刷新时展示专业、准确的 sync-pending 提示；验证：`ContextBar.test.tsx`。
- [x] 2.3 保证自动触发压缩时幕布上的开始/完成文案与真实 lifecycle 一致，不因 payload-less completion 或 source continuity 丢失而误导用户；验证：`useThreadTurnEvents.test.tsx` 与相关 compaction tests。

## 3. Verification Gate

- [x] 3.1 运行 `openspec validate --all --strict --no-interactive`，确认 delta artifacts 合法。
- [x] 3.2 运行 compaction 相关 focused Vitest suites，覆盖 tooltip、幕布和历史恢复边界。
- [x] 3.3 运行 `npm run lint` 与 `npm run typecheck`，确认前端实现无回归。
