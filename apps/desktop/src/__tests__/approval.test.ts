import { describe, expect, it } from "vitest";
import { applicationActions, AGENT_STATE_LABELS, isAgentRunning } from "@job-hunter/shared";
import type { ApplicationStatus } from "@job-hunter/types";

describe("approval workflow guards", () => {
  const statuses: ApplicationStatus[] = ["DISCOVERED", "ANALYZED", "SHORTLISTED", "READY_FOR_REVIEW", "APPROVED", "APPLYING", "APPLIED", "MANUAL_ACTION_REQUIRED", "WAITING_FOR_USER", "REJECTED", "INTERVIEW", "OFFER", "WITHDRAWN"];

  it("never offers an automatic apply action without prior approval", () => {
    for (const s of statuses) {
      const a = applicationActions(s, false);
      if (a.canApply) expect(s).toBe("APPROVED");
    }
  });

  it("disables agent-driven actions while the agent is busy", () => {
    expect(applicationActions("APPROVED", true).canApply).toBe(false);
    expect(applicationActions("WAITING_FOR_USER", true).canAnswer).toBe(false);
  });

  it("always keeps a manual path open for stuck applications", () => {
    for (const s of ["MANUAL_ACTION_REQUIRED", "WAITING_FOR_USER"] as ApplicationStatus[]) {
      expect(applicationActions(s, true).canApplyManually).toBe(true);
      expect(applicationActions(s, true).canMarkApplied).toBe(true);
    }
  });

  it("labels every agent state", () => {
    expect(AGENT_STATE_LABELS.WAITING_FOR_APPROVAL).toMatch(/approval/i);
    expect(isAgentRunning("DISCOVERING")).toBe(true);
    expect(isAgentRunning("WAITING_FOR_APPROVAL")).toBe(false);
  });
});
