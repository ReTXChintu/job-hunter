import { Box, Button, Flex, Input, Link, Text, VStack, Wrap } from "@chakra-ui/react";
import { timeAgo } from "@job-hunter/shared";
import type { Job, JobAnalysis } from "@job-hunter/types";
import { MatchScore, StatusBadge } from "@job-hunter/ui";
import { useQuery } from "@tanstack/react-query";
import { ExternalLink } from "lucide-react";
import { useMemo, useState } from "react";

import { ErrorBox, Loading } from "../components/State";
import { href } from "../route";
import { useSession } from "../session";

type JobFilter = "all" | "relevant" | "unanalyzed";

const FILTERS: { key: JobFilter; label: string }[] = [
  { key: "all", label: "All" },
  { key: "relevant", label: "Relevant" },
  { key: "unanalyzed", label: "Not analyzed" },
];

function JobRow({ job, analysis }: { job: Job; analysis: JobAnalysis | undefined }) {
  return (
    <Flex px={3} py={3} gap={3} borderRadius="md" _hover={{ bg: "bg.muted" }} align={{ base: "flex-start", md: "center" }} direction={{ base: "column", md: "row" }}>
      <Box flex="1" minW={0}>
        {job.applicationId ? (
          <Link href={href({ page: "application", id: job.applicationId })} fontWeight="medium">
            {job.title}
          </Link>
        ) : (
          <Text fontWeight="medium" truncate>
            {job.title}
          </Text>
        )}
        <Text fontSize="xs" color="fg.muted" truncate>
          {job.company}
          {job.location ? ` · ${job.location}` : ""} · {job.source}
        </Text>
      </Box>
      <Flex gap={4} align="center" wrap="wrap">
        <MatchScore score={analysis?.matchScore} relevant={analysis?.relevant} compact />
        <StatusBadge status={job.status} />
        <Text fontSize="xs" color="fg.muted" minW="70px">
          {timeAgo(job.discoveredAt)}
        </Text>
        {job.url ? (
          <Link href={job.url} target="_blank" rel="noreferrer" color="fg.muted" aria-label="Open posting">
            <ExternalLink size={14} />
          </Link>
        ) : null}
      </Flex>
    </Flex>
  );
}

export function JobsPage() {
  const { api } = useSession();
  const jobs = useQuery({ queryKey: ["jobs"], queryFn: () => api.jobs(), refetchInterval: 60_000 });
  const analyses = useQuery({ queryKey: ["analyses"], queryFn: () => api.analyses(), refetchInterval: 60_000 });
  const [filter, setFilter] = useState<JobFilter>("all");
  const [query, setQuery] = useState("");

  const analysisByJob = useMemo(() => new Map((analyses.data ?? []).map((a) => [a.jobId, a])), [analyses.data]);
  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (jobs.data ?? [])
      .filter((job) => {
        const a = analysisByJob.get(job.id);
        if (filter === "relevant") return !!a?.relevant;
        if (filter === "unanalyzed") return !a;
        return true;
      })
      .filter((job) => !q || `${job.title} ${job.company} ${job.location}`.toLowerCase().includes(q))
      .sort((a, b) => b.discoveredAt.localeCompare(a.discoveredAt));
  }, [jobs.data, analysisByJob, filter, query]);

  return (
    <VStack align="stretch" gap={5}>
      <Box>
        <Text fontSize="2xl" fontWeight="semibold">
          Jobs
        </Text>
        <Text color="fg.muted">Every job the agent has discovered, newest first.</Text>
      </Box>

      <Flex gap={3} direction={{ base: "column", md: "row" }} align={{ base: "stretch", md: "center" }} justify="space-between">
        <Wrap gap={2}>
          {FILTERS.map((f) => (
            <Button key={f.key} size="xs" variant={f.key === filter ? "solid" : "outline"} colorPalette={f.key === filter ? "brand" : "gray"} onClick={() => setFilter(f.key)}>
              {f.label}
            </Button>
          ))}
        </Wrap>
        <Input size="sm" maxW={{ md: "260px" }} placeholder="Search company, role, location" value={query} onChange={(e) => setQuery(e.target.value)} />
      </Flex>

      {jobs.isPending ? (
        <Loading />
      ) : jobs.isError ? (
        <ErrorBox error={jobs.error} onRetry={() => void jobs.refetch()} />
      ) : visible.length === 0 ? (
        <Box borderWidth="1px" borderColor="border.muted" borderRadius="lg" p={8} textAlign="center">
          <Text color="fg.muted">{jobs.data.length === 0 ? "No jobs yet. Start a job hunt from the desktop app." : "Nothing matches this filter."}</Text>
        </Box>
      ) : (
        <Box borderWidth="1px" borderColor="border.muted" borderRadius="lg" bg="bg.panel" p={2}>
          <VStack align="stretch" gap={0}>
            {visible.map((job) => (
              <JobRow key={job.id} job={job} analysis={analysisByJob.get(job.id)} />
            ))}
          </VStack>
        </Box>
      )}
    </VStack>
  );
}
