// @vitest-environment jsdom
import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { WorkspaceInfo } from "@/types";
import { useMailDrivenSessionContinuation } from "./useMailDrivenSessionContinuation";

const checkEmailInboxMock = vi.fn();
const claimNextEmailMailCommandMock = vi.fn();
const completeEmailMailCommandMock = vi.fn();
const getEmailInboundListenerStatusMock = vi.fn();

vi.mock("@/services/tauri", () => ({
  checkEmailInbox: (...args: unknown[]) => checkEmailInboxMock(...args),
  claimNextEmailMailCommand: (...args: unknown[]) => claimNextEmailMailCommandMock(...args),
  completeEmailMailCommand: (...args: unknown[]) => completeEmailMailCommandMock(...args),
  getEmailInboundListenerStatus: (...args: unknown[]) => getEmailInboundListenerStatusMock(...args),
}));

const workspace: WorkspaceInfo = {
  id: "workspace-1",
  name: "Workspace",
  path: "/tmp/workspace",
  connected: true,
  settings: {
    sidebarCollapsed: false,
  },
};

async function flushMicrotasks(times = 5) {
  for (let index = 0; index < times; index += 1) {
    await Promise.resolve();
  }
}

describe("useMailDrivenSessionContinuation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getEmailInboundListenerStatusMock.mockResolvedValue({
      enabled: true,
      readOnly: true,
      connectionState: "ready",
      lastCheckedAt: null,
      nextCheckAt: null,
      acceptedCount: 0,
      queuedCount: 0,
      needsConfirmationCount: 0,
      rejectedCount: 0,
      ignoredCount: 0,
      pollingIntervalSeconds: 300,
    });
    checkEmailInboxMock.mockResolvedValue({
      checkedAt: "2026-05-21T10:00:00Z",
      readOnly: true,
      scannedCount: 0,
      acceptedCount: 0,
      queuedCount: 0,
      needsConfirmationCount: 0,
      rejectedCount: 0,
      ignoredCount: 0,
      duplicateCount: 0,
    });
    claimNextEmailMailCommandMock.mockResolvedValue({ command: null });
    completeEmailMailCommandMock.mockResolvedValue({ listener: {}, sessions: [], timeline: [] });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("checks the enabled inbox before claiming queued mail commands", async () => {
    const { unmount } = renderHook(() =>
      useMailDrivenSessionContinuation({
        activeWorkspace: workspace,
        sendUserMessageToThread: vi.fn(),
        armMailDrivenCompletionEmail: vi.fn(),
      }),
    );

    await waitFor(() => {
      expect(checkEmailInboxMock).toHaveBeenCalledWith({});
      expect(claimNextEmailMailCommandMock).toHaveBeenCalled();
    });
    expect(getEmailInboundListenerStatusMock).toHaveBeenCalled();
    unmount();
  });

  it("does not check the inbox when inbound listener is disabled", async () => {
    getEmailInboundListenerStatusMock.mockResolvedValueOnce({
      enabled: false,
      readOnly: true,
      connectionState: "disabled",
      lastCheckedAt: null,
      nextCheckAt: null,
      acceptedCount: 0,
      queuedCount: 0,
      needsConfirmationCount: 0,
      rejectedCount: 0,
      ignoredCount: 0,
      pollingIntervalSeconds: 300,
    });

    const { unmount } = renderHook(() =>
      useMailDrivenSessionContinuation({
        activeWorkspace: workspace,
        sendUserMessageToThread: vi.fn(),
        armMailDrivenCompletionEmail: vi.fn(),
      }),
    );

    await waitFor(() => {
      expect(claimNextEmailMailCommandMock).toHaveBeenCalled();
    });
    expect(checkEmailInboxMock).not.toHaveBeenCalled();
    unmount();
  });

  it("honors a 10 second inbox polling interval from listener status", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-05-21T10:00:00Z"));
    getEmailInboundListenerStatusMock.mockResolvedValue({
      enabled: true,
      readOnly: true,
      connectionState: "ready",
      lastCheckedAt: null,
      nextCheckAt: null,
      acceptedCount: 0,
      queuedCount: 0,
      needsConfirmationCount: 0,
      rejectedCount: 0,
      ignoredCount: 0,
      pollingIntervalSeconds: 10,
    });

    const { unmount } = renderHook(() =>
      useMailDrivenSessionContinuation({
        activeWorkspace: workspace,
        sendUserMessageToThread: vi.fn(),
        armMailDrivenCompletionEmail: vi.fn(),
      }),
    );

    await flushMicrotasks();
    expect(checkEmailInboxMock).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(15_000);
    await flushMicrotasks();
    expect(checkEmailInboxMock).toHaveBeenCalledTimes(2);

    unmount();
  });

  it("does not reschedule polling after unmount while a poll is in flight", async () => {
    vi.useFakeTimers();
    let resolveClaim: (value: { command: null }) => void = () => {};
    claimNextEmailMailCommandMock.mockReturnValueOnce(
      new Promise((resolve) => {
        resolveClaim = resolve;
      }),
    );

    const { unmount } = renderHook(() =>
      useMailDrivenSessionContinuation({
        activeWorkspace: workspace,
        sendUserMessageToThread: vi.fn(),
        armMailDrivenCompletionEmail: vi.fn(),
      }),
    );

    await flushMicrotasks();
    expect(claimNextEmailMailCommandMock).toHaveBeenCalled();

    unmount();
    resolveClaim({ command: null });
    await flushMicrotasks();

    expect(vi.getTimerCount()).toBe(0);
  });

  it("settles a claimed command when sending to the session fails", async () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const sendUserMessageToThread = vi.fn().mockRejectedValue(new Error("session unavailable"));
    claimNextEmailMailCommandMock.mockResolvedValueOnce({
      command: {
        id: "cmd-1",
        workspaceId: "workspace-1",
        threadId: "thread-1",
        detail: "继续处理",
      },
    });

    const { unmount } = renderHook(() =>
      useMailDrivenSessionContinuation({
        activeWorkspace: workspace,
        sendUserMessageToThread,
        armMailDrivenCompletionEmail: vi.fn(),
      }),
    );

    await waitFor(() => {
      expect(sendUserMessageToThread).toHaveBeenCalled();
      expect(completeEmailMailCommandMock).toHaveBeenCalledWith({
        commandId: "cmd-1",
        status: "needs_confirmation",
        rejectReason: "send_failed",
      });
    });
    expect(warnSpy).toHaveBeenCalledWith(
      "[mail-session] failed to process queued email command",
      expect.any(Error),
    );
    warnSpy.mockRestore();
    unmount();
  });
});
