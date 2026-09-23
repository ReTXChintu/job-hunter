import { ChakraProvider } from "@chakra-ui/react";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MatchScore, StatusBadge } from "@job-hunter/ui";
import type { AgentEvent } from "@job-hunter/types";
import { ActivityFeed } from "../components/ActivityFeed";
import { system } from "../theme";

function wrap(ui: React.ReactElement) {
  return render(<ChakraProvider value={system}>{ui}</ChakraProvider>);
}

describe("StatusBadge", () => {
  it("renders human labels for statuses", () => {
    wrap(<StatusBadge status="READY_FOR_REVIEW" />);
    expect(screen.getByText("Awaiting approval")).toBeInTheDocument();
  });
});

describe("MatchScore", () => {
  it("shows the percentage or a placeholder", () => {
    wrap(<MatchScore score={87} relevant />);
    expect(screen.getByText("87%")).toBeInTheDocument();
    wrap(<MatchScore score={null} />);
    expect(screen.getByText("Not analyzed")).toBeInTheDocument();
  });
});

describe("ActivityFeed", () => {
  it("lists events and an empty state", () => {
    const events: AgentEvent[] = [
      { id: "1", runId: "r", at: new Date().toISOString(), level: "SUCCESS", kind: "STEP_DONE", message: "Found 18 jobs on LinkedIn", data: null },
      { id: "2", runId: "r", at: new Date().toISOString(), level: "WARN", kind: "MESSAGE", message: "CAPTCHA detected", data: null },
    ];
    wrap(<ActivityFeed events={events} />);
    expect(screen.getByText("Found 18 jobs on LinkedIn")).toBeInTheDocument();
    expect(screen.getByText("CAPTCHA detected")).toBeInTheDocument();
    wrap(<ActivityFeed events={[]} />);
    expect(screen.getByText("No activity yet.")).toBeInTheDocument();
  });
});
