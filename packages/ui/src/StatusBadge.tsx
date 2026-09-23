import { Badge } from "@chakra-ui/react";
import type { ApplicationStatus, JobStatus } from "@job-hunter/types";

const LABELS: Record<string, string> = {
  DISCOVERED: "Discovered",
  ANALYZED: "Analyzed",
  NOT_RELEVANT: "Not relevant",
  SHORTLISTED: "Shortlisted",
  READY_FOR_REVIEW: "Awaiting approval",
  APPROVED: "Approved",
  APPLYING: "Applying",
  APPLIED: "Applied",
  MANUAL_ACTION_REQUIRED: "Manual action",
  WAITING_FOR_USER: "Needs your input",
  REJECTED: "Rejected",
  INTERVIEW: "Interview",
  OFFER: "Offer",
  WITHDRAWN: "Withdrawn",
};

const PALETTE: Record<string, string> = {
  DISCOVERED: "gray",
  ANALYZED: "blue",
  NOT_RELEVANT: "gray",
  SHORTLISTED: "blue",
  READY_FOR_REVIEW: "purple",
  APPROVED: "purple",
  APPLYING: "purple",
  APPLIED: "green",
  MANUAL_ACTION_REQUIRED: "orange",
  WAITING_FOR_USER: "orange",
  REJECTED: "red",
  INTERVIEW: "green",
  OFFER: "green",
  WITHDRAWN: "gray",
};

export function StatusBadge({ status, size = "sm" }: { status: JobStatus | ApplicationStatus; size?: "xs" | "sm" | "md" }) {
  return (
    <Badge colorPalette={PALETTE[status] ?? "gray"} variant="subtle" size={size} textTransform="none" fontWeight="medium" borderRadius="sm">
      {LABELS[status] ?? status}
    </Badge>
  );
}
