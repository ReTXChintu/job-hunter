import { describe, expect, it } from "vitest";
import type { ApplicationListItem, CandidateProfile, JobListItem } from "@job-hunter/types";
import { applicationActions, filterJobs, pendingApplications, profileIssues, sortJobs, splitList, statusTone } from "./index";

function job(over: Partial<JobListItem["job"]>, analysis: Partial<NonNullable<JobListItem["analysis"]>> | null = null): JobListItem {
  const base: JobListItem["job"] = {
    id: over.id ?? "j",
    userId: "u",
    discoveredAt: "2026-09-24T00:00:00Z",
    postedAt: null,
    source: "LinkedIn",
    sourceJobId: null,
    url: "https://x",
    canonicalUrl: null,
    company: "Acme",
    title: "Engineer",
    location: "Remote",
    employmentType: null,
    remote: null,
    salary: null,
    seniority: null,
    description: "",
    requirements: [],
    responsibilities: [],
    skills: ["React"],
    sources: [],
    status: "DISCOVERED",
    runId: null,
    analysisId: null,
    applicationId: null,
    saved: false,
    detailsComplete: true,
    dedupKey: "",
    createdAt: "",
    updatedAt: "",
    ...over,
  };
  return {
    job: base,
    analysis: analysis
      ? {
          id: "a",
          jobId: base.id,
          userId: "u",
          relevant: true,
          matchScore: 80,
          matchedSkills: [],
          missingSkills: [],
          requiredExperienceMet: true,
          seniorityMatch: true,
          locationMatch: true,
          employmentTypeMatch: true,
          salaryAssessment: "",
          requiredQualifications: [],
          niceToHave: [],
          concerns: [],
          importantKeywords: [],
          summary: "",
          runId: null,
          createdAt: "",
          updatedAt: "",
          ...analysis,
        }
      : null,
    applicationStatus: null,
  };
}

describe("job filters", () => {
  const items = [
    job({ id: "1", status: "DISCOVERED" }),
    job({ id: "2", status: "READY_FOR_REVIEW" }, { relevant: true, matchScore: 90 }),
    job({ id: "3", status: "REJECTED" }, { relevant: true, matchScore: 70 }),
    job({ id: "4", status: "MANUAL_ACTION_REQUIRED", company: "Zed" }, { relevant: true, matchScore: 60 }),
    job({ id: "5", status: "NOT_RELEVANT", saved: true }, { relevant: false, matchScore: 20 }),
  ];

  it("filters by status buckets", () => {
    expect(filterJobs(items, "NEW").map((i) => i.job.id)).toEqual(["1"]);
    expect(filterJobs(items, "AWAITING_APPROVAL").map((i) => i.job.id)).toEqual(["2"]);
    expect(filterJobs(items, "RELEVANT").map((i) => i.job.id)).toEqual(["2", "4"]);
    expect(filterJobs(items, "REJECTED").map((i) => i.job.id)).toEqual(["3", "5"]);
    expect(filterJobs(items, "MANUAL_ACTION").map((i) => i.job.id)).toEqual(["4"]);
    expect(filterJobs(items, "SAVED").map((i) => i.job.id)).toEqual(["5"]);
  });

  it("applies a text query", () => {
    expect(filterJobs(items, "ALL", "zed").map((i) => i.job.id)).toEqual(["4"]);
  });

  it("sorts by match score then recency", () => {
    expect(sortJobs(items).map((i) => i.job.id)).toEqual(["2", "3", "4", "5", "1"]);
  });
});

describe("application state helpers", () => {
  it("only allows approval from review and applying after approval", () => {
    expect(applicationActions("READY_FOR_REVIEW", false).canApprove).toBe(true);
    expect(applicationActions("READY_FOR_REVIEW", false).canApply).toBe(false);
    expect(applicationActions("APPROVED", false).canApply).toBe(true);
    expect(applicationActions("APPROVED", true).canApply).toBe(false);
    expect(applicationActions("APPLIED", false).canReject).toBe(false);
    expect(applicationActions("WAITING_FOR_USER", false).canAnswer).toBe(true);
    expect(applicationActions("MANUAL_ACTION_REQUIRED", false).canMarkApplied).toBe(true);
  });

  it("collects pending actions", () => {
    const mk = (status: ApplicationListItem["application"]["status"]): ApplicationListItem => ({
      application: { status } as ApplicationListItem["application"],
      job: job({}).job,
      analysis: null,
    });
    expect(pendingApplications([mk("APPLIED"), mk("READY_FOR_REVIEW"), mk("WAITING_FOR_USER")]).length).toBe(2);
  });

  it("maps statuses to tones", () => {
    expect(statusTone("APPLIED")).toBe("success");
    expect(statusTone("MANUAL_ACTION_REQUIRED")).toBe("warning");
    expect(statusTone("REJECTED")).toBe("danger");
  });
});

describe("candidate profile", () => {
  it("reports missing required fields", () => {
    const profile = {
      personal: { name: "", email: "bad", phone: "", location: "", linkedin: "", github: "", portfolio: "", currentTitle: "" },
      preferences: { targetRoles: [] },
      skills: { frontend: [], backend: [], database: [], devops: [], cloud: [], testing: [], other: [] },
    } as unknown as CandidateProfile;
    const issues = profileIssues(profile);
    expect(issues).toContain("Add your name");
    expect(issues).toContain("Email address looks invalid");
    expect(issues).toContain("Add at least one target role");
    expect(issues).toContain("Add your skills");
  });

  it("splits comma and newline lists without duplicates", () => {
    expect(splitList("React, Node.js\nReact,, ")).toEqual(["React", "Node.js"]);
  });
});
