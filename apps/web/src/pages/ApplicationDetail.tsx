import { Alert, Badge, Box, Button, Flex, Link, SimpleGrid, Text, VStack, Wrap } from "@chakra-ui/react";
import { formatDateTime, STATUS_LABELS, timeAgo } from "@job-hunter/shared";
import { KeyValue, MatchScore, StatusBadge } from "@job-hunter/ui";
import { useQuery } from "@tanstack/react-query";
import { ArrowLeft, ExternalLink } from "lucide-react";

import { ErrorBox, Loading, Panel } from "../components/State";
import { href } from "../route";
import { useSession } from "../session";

function Tags({ items, palette }: { items: string[]; palette: string }) {
  if (items.length === 0) return <Text color="fg.muted">None</Text>;
  return (
    <Wrap gap={1.5}>
      {items.map((s) => (
        <Badge key={s} colorPalette={palette} variant="subtle" textTransform="none" fontWeight="normal">
          {s}
        </Badge>
      ))}
    </Wrap>
  );
}

export function ApplicationDetailPage({ id }: { id: string }) {
  const { api } = useSession();
  const detail = useQuery({ queryKey: ["application", id], queryFn: () => api.application(id), refetchInterval: 30_000 });

  const back = (
    <Link href={href({ page: "applications" })} fontSize="sm" color="fg.muted" display="inline-flex" alignItems="center" gap={1}>
      <ArrowLeft size={14} /> Applications
    </Link>
  );

  if (detail.isPending) return <Loading />;
  if (detail.isError)
    return (
      <VStack align="stretch">
        {back}
        <ErrorBox error={detail.error} onRetry={() => void detail.refetch()} />
      </VStack>
    );

  const { application: app, job, analysis } = detail.data;
  const history = [...app.statusHistory].sort((a, b) => b.at.localeCompare(a.at));
  const jobUrl = app.applicationUrl || job.url;

  return (
    <VStack align="stretch" gap={5}>
      {back}

      <Flex justify="space-between" gap={4} align={{ base: "flex-start", md: "center" }} direction={{ base: "column", md: "row" }}>
        <Box minW={0}>
          <Text fontSize="2xl" fontWeight="semibold">
            {job.title}
          </Text>
          <Text color="fg.muted">
            {job.company}
            {job.location ? ` · ${job.location}` : ""}
          </Text>
        </Box>
        <Flex gap={3} align="center" wrap="wrap">
          <StatusBadge status={app.status} size="md" />
          {jobUrl ? (
            <Button asChild size="sm" variant="outline">
              <a href={jobUrl} target="_blank" rel="noreferrer">
                View posting <ExternalLink size={14} />
              </a>
            </Button>
          ) : null}
        </Flex>
      </Flex>

      {app.failureReason ? (
        <Alert.Root status="warning" borderRadius="md">
          <Alert.Indicator />
          <Alert.Content>
            <Alert.Title>Needs attention</Alert.Title>
            <Alert.Description>{app.failureReason}</Alert.Description>
          </Alert.Content>
        </Alert.Root>
      ) : null}

      {app.pendingQuestions.length > 0 ? (
        <Panel title="Questions waiting for your answer">
          <VStack align="stretch" gap={2}>
            {app.pendingQuestions.map((q) => (
              <Text key={q.id} fontSize="sm">
                • {q.question}
              </Text>
            ))}
            <Text fontSize="xs" color="fg.muted">
              Answer these from the desktop or mobile app.
            </Text>
          </VStack>
        </Panel>
      ) : null}

      <SimpleGrid columns={{ base: 1, lg: 3 }} gap={4} alignItems="start">
        <Box gridColumn={{ lg: "span 2" }}>
          <VStack align="stretch" gap={4}>
            <Panel title="Match analysis">
              {analysis ? (
                <VStack align="stretch" gap={4}>
                  <Box maxW="320px">
                    <MatchScore score={analysis.matchScore} relevant={analysis.relevant} />
                  </Box>
                  {analysis.summary ? <Text fontSize="sm">{analysis.summary}</Text> : null}
                  <KeyValue label="Matched skills">
                    <Tags items={analysis.matchedSkills} palette="green" />
                  </KeyValue>
                  <KeyValue label="Missing skills">
                    <Tags items={analysis.missingSkills} palette="orange" />
                  </KeyValue>
                  {analysis.concerns.length > 0 ? (
                    <KeyValue label="Concerns">
                      <VStack align="stretch" gap={1}>
                        {analysis.concerns.map((c) => (
                          <Text key={c} fontSize="sm">
                            • {c}
                          </Text>
                        ))}
                      </VStack>
                    </KeyValue>
                  ) : null}
                </VStack>
              ) : (
                <Text color="fg.muted">This job hasn&apos;t been analyzed yet.</Text>
              )}
            </Panel>

            {app.answers.length > 0 ? (
              <Panel title="Answers submitted">
                <VStack align="stretch" gap={3}>
                  {app.answers.map((a) => (
                    <Box key={a.question}>
                      <Text fontSize="xs" color="fg.muted">
                        {a.question}
                      </Text>
                      <Text fontSize="sm">{a.answer || "—"}</Text>
                    </Box>
                  ))}
                </VStack>
              </Panel>
            ) : null}

            {job.description ? (
              <Panel title="Job description">
                <Text fontSize="sm" whiteSpace="pre-wrap" maxH="480px" overflowY="auto">
                  {job.description}
                </Text>
              </Panel>
            ) : null}
          </VStack>
        </Box>

        <VStack align="stretch" gap={4}>
          <Panel title="Details">
            <VStack align="stretch" gap={3}>
              <KeyValue label="Source">{job.source || "—"}</KeyValue>
              {job.employmentType ? <KeyValue label="Employment type">{job.employmentType}</KeyValue> : null}
              {job.remote ? <KeyValue label="Remote">{job.remote}</KeyValue> : null}
              {job.salary ? <KeyValue label="Salary">{job.salary}</KeyValue> : null}
              <KeyValue label="Approved">{app.approvedAt ? formatDateTime(app.approvedAt) : "Not yet"}</KeyValue>
              <KeyValue label="Applied">{app.appliedAt ? formatDateTime(app.appliedAt) : "Not yet"}</KeyValue>
              <KeyValue label="Last update">{timeAgo(app.updatedAt)}</KeyValue>
              {app.notes ? <KeyValue label="Notes">{app.notes}</KeyValue> : null}
            </VStack>
          </Panel>

          <Panel title="History">
            {history.length === 0 ? (
              <Text color="fg.muted">No status changes recorded.</Text>
            ) : (
              <VStack align="stretch" gap={3}>
                {history.map((h, i) => (
                  <Box key={`${h.at}-${i}`} borderLeftWidth="2px" borderColor={i === 0 ? "brand.solid" : "border.muted"} pl={3}>
                    <Text fontSize="sm" fontWeight="medium">
                      {STATUS_LABELS[h.status] ?? h.status}
                    </Text>
                    <Text fontSize="xs" color="fg.muted">
                      {formatDateTime(h.at)}
                    </Text>
                    {h.reason ? (
                      <Text fontSize="xs" color="fg.muted">
                        {h.reason}
                      </Text>
                    ) : null}
                  </Box>
                ))}
              </VStack>
            )}
          </Panel>
        </VStack>
      </SimpleGrid>
    </VStack>
  );
}
