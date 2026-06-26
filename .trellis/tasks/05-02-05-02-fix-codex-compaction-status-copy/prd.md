# Fix Codex Compaction Status Copy

## Goal

修正 Codex 自动压缩在前端 tooltip / 幕布上的状态语义与文案，让“压缩生命周期”与“背景信息用量快照”不再互相误导。

## Requirements

- Tooltip 不能再只因为历史上出现过 compaction message 就永久显示 completed。
- 自动压缩完成但 usage snapshot 尚未刷新时，需要显示明确的 sync-pending 提示。
- Codex 自动触发压缩的开始/完成文案必须与真实 lifecycle 对齐。
- 现有 backend 自动压缩阈值逻辑不变。

## Acceptance Criteria

- 自动压缩完成后，如果最新 usage 仍是压缩前快照，tooltip 展示准确过渡文案而不是误导性的“已压缩”。
- 历史恢复后，当前 tooltip 状态由 thread lifecycle metadata 决定，而不是由 preserved historical message 决定。
- focused Vitest、`npm run lint`、`npm run typecheck` 通过。

## Technical Notes

- 关联 OpenSpec change：`fix-codex-compaction-status-copy`
- 主要触点：
  - `src/features/threads/hooks/useThreadsReducer.ts`
  - `src/features/threads/hooks/useThreadTurnEvents.ts`
  - `src/features/composer/components/Composer.tsx`
  - `src/features/composer/components/ChatInputBox/ContextBar.tsx`
  - `src/i18n/locales/zh.part2.ts`
  - `src/i18n/locales/en.part2.ts`
