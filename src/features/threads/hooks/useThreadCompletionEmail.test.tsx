// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { MutableRefObject } from "react";
import type { ConversationItem } from "../../../types";
import { sendConversationCompletionEmail } from "../../../services/tauri";
import { useThreadCompletionEmail } from "./useThreadCompletionEmail";

vi.mock("../../../services/tauri", () => ({
  sendConversationCompletionEmail: vi.fn(),
}));

vi.mock("../../../services/toasts", () => ({
  pushErrorToast: vi.fn(),
}));

function mutableRef<T>(current: T): MutableRefObject<T> {
  return { current };
}

const baseMetadata = {
  workspaceId: "ws-1",
  workspaceName: "ccgui",
  workspacePath: "/tmp/project",
  threadId: "codex:first",
  threadName: "Codex first",
  turnId: "turn-1",
  engine: "codex" as const,
};

describe("useThreadCompletionEmail", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    vi.mocked(sendConversationCompletionEmail).mockResolvedValue({
      provider: "custom",
      acceptedRecipients: ["dev@example.com"],
      durationMs: 12,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("retries when Codex completion settles before the final assistant message is visible", async () => {
    const activeTurnIdByThreadRef = mutableRef<Record<string, string | null>>({
      "codex:first": "turn-1",
    });
    const userItem: ConversationItem = {
      id: "user-1",
      kind: "message",
      role: "user",
      text: "完成后发邮件",
    };
    const itemsByThreadRef = mutableRef<Record<string, ConversationItem[]>>({
      "codex:first": [userItem],
    });
    const setActiveTurnId = vi.fn();
    const onDebug = vi.fn();
    const { result } = renderHook(() =>
      useThreadCompletionEmail({
        activeThreadId: "codex:first",
        activeTurnIdByThreadRef,
        itemsByThreadRef,
        resolveCanonicalThreadId: (threadId) => threadId,
        setActiveTurnId,
        getCompletionEmailMetadata: () => baseMetadata,
        onDebug,
      }),
    );

    act(() => {
      result.current.toggleCompletionEmailIntent("codex:first");
    });

    expect(result.current.completionEmailIntentByThread["codex:first"]).toBeDefined();

    act(() => {
      result.current.settleCompletionEmailIntent("ws-1", "codex:first", "turn-1", "completed");
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });

    expect(sendConversationCompletionEmail).not.toHaveBeenCalled();
    expect(onDebug).toHaveBeenCalledWith(
      expect.objectContaining({
        label: "completion-email/build-retry",
        payload: expect.objectContaining({
          reason: "missing_assistant_message",
          attempt: 1,
        }),
      }),
    );

    itemsByThreadRef.current = {
      "codex:first": [
        userItem,
        {
          id: "assistant-1",
          kind: "message",
          role: "assistant",
          text: "Codex 已完成。",
          isFinal: true,
          finalCompletedAt: Date.now() + 1,
        },
      ],
    };

    await act(async () => {
      await vi.advanceTimersByTimeAsync(250);
    });

    expect(sendConversationCompletionEmail).toHaveBeenCalledTimes(1);
    expect(sendConversationCompletionEmail).toHaveBeenCalledWith(
      expect.objectContaining({
        workspaceId: "ws-1",
        threadId: "codex:first",
        turnId: "turn-1",
        textBody: expect.stringContaining("Codex 已完成。"),
      }),
    );
  });

  it("binds mail-driven completion email to the next turn instead of reusing the previous final message", async () => {
    vi.setSystemTime(10_000);
    const activeTurnIdByThreadRef = mutableRef<Record<string, string | null>>({
      "codex:first": "turn-1",
    });
    const oldUserItem: ConversationItem = {
      id: "user-1",
      kind: "message",
      role: "user",
      text: "项目分析",
    };
    const oldAssistantItem: ConversationItem = {
      id: "assistant-1",
      kind: "message",
      role: "assistant",
      text: "上一轮项目分析结果。",
      isFinal: true,
      finalCompletedAt: 1_000,
    };
    const itemsByThreadRef = mutableRef<Record<string, ConversationItem[]>>({
      "codex:first": [oldUserItem, oldAssistantItem],
    });
    const setActiveTurnId = vi.fn((threadId: string, turnId: string | null) => {
      activeTurnIdByThreadRef.current[threadId] = turnId;
    });
    const onDebug = vi.fn();
    const { result } = renderHook(() =>
      useThreadCompletionEmail({
        activeThreadId: "codex:first",
        activeTurnIdByThreadRef,
        itemsByThreadRef,
        resolveCanonicalThreadId: (threadId) => threadId,
        setActiveTurnId,
        getCompletionEmailMetadata: (_workspaceId, _threadId, turnId) => ({
          ...baseMetadata,
          turnId,
        }),
        onDebug,
      }),
    );

    act(() => {
      result.current.armMailDrivenCompletionEmail("codex:first", null);
    });

    act(() => {
      result.current.settleCompletionEmailIntent("ws-1", "codex:first", "turn-1", "completed");
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(sendConversationCompletionEmail).not.toHaveBeenCalled();

    act(() => {
      result.current.setActiveTurnIdWithCompletionEmail("codex:first", "turn-2");
    });
    act(() => {
      result.current.settleCompletionEmailIntent("ws-1", "codex:first", "turn-2", "completed");
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(sendConversationCompletionEmail).not.toHaveBeenCalled();
    expect(onDebug).toHaveBeenCalledWith(
      expect.objectContaining({
        label: "completion-email/build-retry",
        payload: expect.objectContaining({
          turnId: "turn-2",
          reason: "missing_assistant_message",
        }),
      }),
    );

    itemsByThreadRef.current = {
      "codex:first": [
        oldUserItem,
        oldAssistantItem,
        {
          id: "user-2",
          kind: "message",
          role: "user",
          text: "继续更新项目文档",
        },
        {
          id: "assistant-2",
          kind: "message",
          role: "assistant",
          text: "已完成最新一轮文档更新。",
          isFinal: true,
          finalCompletedAt: 10_100,
        },
      ],
    };

    await act(async () => {
      await vi.advanceTimersByTimeAsync(250);
    });

    expect(sendConversationCompletionEmail).toHaveBeenCalledTimes(1);
    expect(sendConversationCompletionEmail).toHaveBeenCalledWith(
      expect.objectContaining({
        turnId: "turn-2",
        textBody: expect.stringContaining("继续更新项目文档"),
        summary: expect.stringContaining("已完成最新一轮文档更新"),
      }),
    );
    expect(sendConversationCompletionEmail).not.toHaveBeenCalledWith(
      expect.objectContaining({
        turnId: "turn-1",
      }),
    );
  });
});
