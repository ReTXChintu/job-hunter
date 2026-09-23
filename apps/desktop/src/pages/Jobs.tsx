import { Box, Button, HStack, Input, Table, Tabs, Text } from "@chakra-ui/react";
import { useNavigate } from "@tanstack/react-router";
import { filterJobs, JOB_FILTERS, sortJobs, timeAgo, type JobFilter } from "@job-hunter/shared";
import { EmptyState, MatchScore, StatusBadge } from "@job-hunter/ui";
import { Bookmark, Search } from "lucide-react";
import { useMemo, useState } from "react";
import { Chips, ErrorBanner, PageHeader } from "../components/common";
import { useDiscoverJobs, useJobs, useAgentStatus } from "../lib/queries";
import { isAgentRunning } from "@job-hunter/shared";

export function JobsPage() {
  const jobs = useJobs();
  const agent = useAgentStatus();
  const discover = useDiscoverJobs();
  const navigate = useNavigate();
  const [filter, setFilter] = useState<JobFilter>("ALL");
  const [query, setQuery] = useState("");
  const items = useMemo(() => sortJobs(filterJobs(jobs.data ?? [], filter, query)), [jobs.data, filter, query]);
  const counts = useMemo(() => Object.fromEntries(JOB_FILTERS.map((f) => [f.key, filterJobs(jobs.data ?? [], f.key).length])) as Record<JobFilter, number>, [jobs.data]);
  const running = agent.data ? isAgentRunning(agent.data.state) : false;

  return (
    <Box>
      <PageHeader
        title="Jobs"
        subtitle={`${jobs.data?.length ?? 0} jobs discovered`}
        actions={
          <Button variant="subtle" onClick={() => discover.mutate(undefined)} loading={discover.isPending} disabled={running}>
            <Search size={16} /> Discover jobs
          </Button>
        }
      />
      <ErrorBanner error={jobs.error ?? discover.error} onRetry={() => jobs.refetch()} />
      <HStack mb={4} gap={4} align="center">
        <Tabs.Root value={filter} onValueChange={(e) => setFilter(e.value as JobFilter)} variant="subtle" size="sm" flex="1">
          <Tabs.List>
            {JOB_FILTERS.map((f) => (
              <Tabs.Trigger key={f.key} value={f.key}>
                {f.label}
                <Text as="span" ml={1.5} fontSize="xs" color="fg.muted">
                  {counts[f.key]}
                </Text>
              </Tabs.Trigger>
            ))}
          </Tabs.List>
        </Tabs.Root>
        <Input size="sm" maxW="260px" placeholder="Search title, company, skill" value={query} onChange={(e) => setQuery(e.target.value)} />
      </HStack>
      {items.length === 0 ? (
        <EmptyState title={jobs.data?.length ? "No jobs match this filter" : "No jobs yet"} description={jobs.data?.length ? "Try another filter or search." : "Start a job hunt from the dashboard or use Discover jobs to search your enabled sources."} />
      ) : (
        <Table.Root size="sm" interactive borderWidth="1px" borderColor="border.muted" borderRadius="md">
          <Table.Header>
            <Table.Row>
              <Table.ColumnHeader w="38%">Job</Table.ColumnHeader>
              <Table.ColumnHeader>Location</Table.ColumnHeader>
              <Table.ColumnHeader>Posted</Table.ColumnHeader>
              <Table.ColumnHeader>Source</Table.ColumnHeader>
              <Table.ColumnHeader>Match</Table.ColumnHeader>
              <Table.ColumnHeader>Technologies</Table.ColumnHeader>
              <Table.ColumnHeader>Status</Table.ColumnHeader>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {items.map((item) => (
              <Table.Row key={item.job.id} cursor="pointer" onClick={() => navigate({ to: "/jobs/$jobId", params: { jobId: item.job.id } })}>
                <Table.Cell>
                  <HStack gap={2} align="flex-start">
                    {item.job.saved ? <Bookmark size={14} style={{ marginTop: 3 }} /> : null}
                    <Box minW={0}>
                      <Text fontWeight="semibold" truncate>
                        {item.job.title}
                      </Text>
                      <Text fontSize="xs" color="fg.muted" truncate>
                        {item.job.company}
                        {item.job.remote ? ` · ${item.job.remote}` : ""}
                      </Text>
                    </Box>
                  </HStack>
                </Table.Cell>
                <Table.Cell>
                  <Text fontSize="sm" truncate maxW="160px">
                    {item.job.location || "—"}
                  </Text>
                </Table.Cell>
                <Table.Cell>
                  <Text fontSize="sm" color="fg.muted">
                    {item.job.postedAt ?? timeAgo(item.job.discoveredAt)}
                  </Text>
                </Table.Cell>
                <Table.Cell>
                  <Text fontSize="sm">{item.job.source}</Text>
                  {item.job.sources.length > 1 ? (
                    <Text fontSize="xs" color="fg.muted">
                      +{item.job.sources.length - 1} more
                    </Text>
                  ) : null}
                </Table.Cell>
                <Table.Cell>
                  <MatchScore score={item.analysis?.matchScore} relevant={item.analysis?.relevant} compact />
                </Table.Cell>
                <Table.Cell maxW="220px">
                  <Chips items={(item.analysis?.matchedSkills.length ? item.analysis.matchedSkills : item.job.skills).slice(0, 4)} palette="blue" max={4} />
                </Table.Cell>
                <Table.Cell>
                  <StatusBadge status={item.job.status} />
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table.Root>
      )}
    </Box>
  );
}
