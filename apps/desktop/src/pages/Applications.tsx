import { Box, HStack, Table, Tabs, Text } from "@chakra-ui/react";
import { useNavigate } from "@tanstack/react-router";
import { timeAgo } from "@job-hunter/shared";
import type { ApplicationStatus } from "@job-hunter/types";
import { EmptyState, MatchScore, StatusBadge } from "@job-hunter/ui";
import { useMemo, useState } from "react";
import { ErrorBanner, PageHeader } from "../components/common";
import { useApplications } from "../lib/queries";

type Bucket = "PENDING" | "ALL" | "APPLIED" | "REJECTED";
const BUCKETS: { key: Bucket; label: string; statuses?: ApplicationStatus[] }[] = [
  { key: "PENDING", label: "Needs attention", statuses: ["READY_FOR_REVIEW", "WAITING_FOR_USER", "MANUAL_ACTION_REQUIRED", "APPROVED", "APPLYING"] },
  { key: "APPLIED", label: "Applied", statuses: ["APPLIED", "INTERVIEW", "OFFER"] },
  { key: "REJECTED", label: "Rejected / withdrawn", statuses: ["REJECTED", "WITHDRAWN"] },
  { key: "ALL", label: "All" },
];

export function ApplicationsPage() {
  const apps = useApplications();
  const navigate = useNavigate();
  const [bucket, setBucket] = useState<Bucket>("PENDING");
  const items = useMemo(() => {
    const b = BUCKETS.find((x) => x.key === bucket)!;
    return (apps.data ?? []).filter((i) => !b.statuses || b.statuses.includes(i.application.status));
  }, [apps.data, bucket]);
  const awaiting = (apps.data ?? []).filter((i) => i.application.status === "READY_FOR_REVIEW").length;

  return (
    <Box>
      <PageHeader title="Applications" subtitle={awaiting > 0 ? `${awaiting} application${awaiting === 1 ? "" : "s"} waiting for your approval` : "Every submission needs your explicit approval."} />
      <ErrorBanner error={apps.error} onRetry={() => apps.refetch()} />
      <Tabs.Root value={bucket} onValueChange={(e) => setBucket(e.value as Bucket)} variant="subtle" size="sm" mb={4}>
        <Tabs.List>
          {BUCKETS.map((b) => (
            <Tabs.Trigger key={b.key} value={b.key}>
              {b.label}
              <Text as="span" ml={1.5} fontSize="xs" color="fg.muted">
                {(apps.data ?? []).filter((i) => !b.statuses || b.statuses.includes(i.application.status)).length}
              </Text>
            </Tabs.Trigger>
          ))}
        </Tabs.List>
      </Tabs.Root>
      {items.length === 0 ? (
        <EmptyState title="Nothing here" description={bucket === "PENDING" ? "Applications prepared by the agent will show up here for your review." : "No applications in this bucket."} />
      ) : (
        <Table.Root size="sm" interactive borderWidth="1px" borderColor="border.muted" borderRadius="md">
          <Table.Header>
            <Table.Row>
              <Table.ColumnHeader w="40%">Job</Table.ColumnHeader>
              <Table.ColumnHeader>Match</Table.ColumnHeader>
              <Table.ColumnHeader>Status</Table.ColumnHeader>
              <Table.ColumnHeader>Updated</Table.ColumnHeader>
              <Table.ColumnHeader>Note</Table.ColumnHeader>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {items.map((item) => (
              <Table.Row key={item.application.id} cursor="pointer" onClick={() => navigate({ to: "/applications/$applicationId", params: { applicationId: item.application.id } })}>
                <Table.Cell>
                  <Text fontWeight="semibold">{item.job.title}</Text>
                  <Text fontSize="xs" color="fg.muted">
                    {item.job.company} · {item.job.location || item.job.source}
                  </Text>
                </Table.Cell>
                <Table.Cell>
                  <MatchScore score={item.analysis?.matchScore} relevant={item.analysis?.relevant} compact />
                </Table.Cell>
                <Table.Cell>
                  <StatusBadge status={item.application.status} />
                </Table.Cell>
                <Table.Cell>
                  <Text fontSize="sm" color="fg.muted">
                    {timeAgo(item.application.updatedAt)}
                  </Text>
                </Table.Cell>
                <Table.Cell>
                  <HStack>
                    <Text fontSize="xs" color={item.application.failureReason ? "orange.fg" : "fg.muted"} truncate maxW="320px">
                      {item.application.failureReason ?? (item.application.pendingQuestions.length ? `${item.application.pendingQuestions.length} question(s) need answers` : item.application.evidence ? "Submission confirmed" : "")}
                    </Text>
                  </HStack>
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table.Root>
      )}
    </Box>
  );
}
