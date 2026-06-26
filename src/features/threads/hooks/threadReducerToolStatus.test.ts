import { describe, expect, it } from "vitest";
import {
  isCompletedToolStatus,
  isFailedToolStatus,
  isPendingToolStatus,
  shouldFinalizeToolStatus,
} from "./threadReducerToolStatus";

describe("threadReducerToolStatus", () => {
  it("classifies terminal failed statuses", () => {
    expect(isFailedToolStatus("failed")).toBe(true);
    expect(isFailedToolStatus("timed_out")).toBe(true);
    expect(isFailedToolStatus("cancelled")).toBe(true);
    expect(shouldFinalizeToolStatus("failed")).toBe(false);
  });

  it("classifies terminal completed statuses", () => {
    expect(isCompletedToolStatus("completed")).toBe(true);
    expect(isCompletedToolStatus("success")).toBe(true);
    expect(isCompletedToolStatus("finished")).toBe(true);
    expect(shouldFinalizeToolStatus("completed")).toBe(false);
  });

  it("classifies pending statuses as finalizable", () => {
    expect(isPendingToolStatus("running")).toBe(true);
    expect(isPendingToolStatus("in_progress")).toBe(true);
    expect(isPendingToolStatus("queued")).toBe(true);
    expect(shouldFinalizeToolStatus("running")).toBe(true);
  });

  it("finalizes empty status but preserves unknown non-pending status", () => {
    expect(shouldFinalizeToolStatus("")).toBe(true);
    expect(shouldFinalizeToolStatus(undefined)).toBe(true);
    expect(shouldFinalizeToolStatus("waiting_for_user")).toBe(false);
  });
});
