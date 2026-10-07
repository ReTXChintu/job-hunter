import { Badge, Box, Button, HStack, Input, Table, Tabs, Text } from "@chakra-ui/react";
import { useNavigate } from "@tanstack/react-router";
import { filterJobs, JOB_FILTERS, sortJobs, timeAgo, type JobFilter } from "@job-hunter/shared";
import { EmptyState, MatchScore, StatusBadge } from "@job-hunter/ui";
import { Bookmark, FilePlus, Link2, Mail, Search, Send, Wand2 } from "lucide-react";
import { useMemo, useState } from "react";
import { Chips, ErrorBanner, PageHeader } from "../components/common";
import { JobLinkDialog } from "../components/JobLinkDialog";
import { SharedJobDialog } from "../components/SharedJobDialog";
import { useDiscoverJobs, useJobs, useAgentStatus, usePrepareApplications, useStartJobHunt } from "../lib/queries";
import { isAgentRunning } from "@job-hunter/shared";

export function JobsPage() {
  const jobs = useJobs();
  const agent = useAgentStatus();
  const discover = useDiscoverJobs();
  const prepare = usePrepareApplications();
  const huntPosts = useStartJobHunt();
  const [sharing, setSharing] = useState(false);
  const [linking, setLinking] = useState(false);
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
          <HStack gap={2}>
            <Button colorPalette="brand" onClick={() => setLinking(true)} disabled={running} title="Paste a job link and start applying">
              <Link2 size={16} /> Apply from a link
            </Button>
            <Button variant="outline" onClick={() => setSharing(true)} disabled={running} title="Paste a job post or add screenshots/PDFs; get a resume and a Gmail draft">
              <FilePlus size={16} /> Add a job I found
            </Button>
            <Button
              variant="outline"
              onClick={() => huntPosts.mutate({ sources: ["Telegram"] })}
              loading={huntPosts.isPending && huntPosts.variables?.sources?.[0] === "Telegram"}
              disabled={running}
              title="Read your Telegram channels since the last scan and prepare the job posts for review"
            >
              <Send size={16} /> Scan Telegram
            </Button>
            <Button
              variant="outline"
              onClick={() => huntPosts.mutate({ sources: ["LinkedIn Posts"] })}
              loading={huntPosts.isPending}
              disabled={running}
              title="Search LinkedIn posts that ask for resumes by email, then analyze them and prepare applications for your review"
            >
              <Mail size={16} /> Find hiring posts
            </Button>
            <Button
              variant="outline"
              onClick={() => prepare.mutate()}
              loading={prepare.isPending}
              disabled={running || counts.RELEVANT === 0}
              title="Generate resumes and prepare applications for the best jobs already found (no new search); they wait for your approval"
            >
              <Wand2 size={16} /> Prepare applications
            </Button>
            <Button
              variant="subtle"
              onClick={() => discover.mutate(undefined)}
              loading={discover.isPending}
              disabled={running}
              title="Search your enabled sources and score the jobs only. No resumes or applications are prepared; use Start job hunt for that."
            >
              <Search size={16} /> Search only
            </Button>
          </HStack>
        }
      />
      <ErrorBanner error={jobs.error ?? discover.error ?? prepare.error ?? huntPosts.error} onRetry={() => jobs.refetch()} />
      <SharedJobDialog open={sharing} onClose={() => setSharing(false)} />
      <JobLinkDialog open={linking} onClose={() => setLinking(false)} />
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
        <EmptyState title={jobs.data?.length ? "No jobs match this filter" : "No jobs yet"} description={jobs.data?.length ? "Try another filter or search." : "Start a job hunt from the dashboard, or use Search only to find and score jobs without preparing applications."} />
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
                      <HStack gap={1.5}>
                        <Text fontWeight="semibold" truncate>
                          {item.job.title}
                        </Text>
                        {item.job.applyEmail ? (
                          <Badge size="xs" colorPalette="blue" variant="subtle" title={`Apply by email to ${item.job.applyEmail}`}>
                            Email
                          </Badge>
                        ) : null}
                      </HStack>
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
