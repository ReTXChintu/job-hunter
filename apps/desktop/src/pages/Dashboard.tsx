import { Box, Button, Grid, GridItem, Heading, HStack, SimpleGrid, Spinner, Table, Text, VStack } from "@chakra-ui/react";
import { Link, useNavigate } from "@tanstack/react-router";
import { AGENT_STATE_LABELS, greeting, isAgentRunning, timeAgo } from "@job-hunter/shared";
import { MatchScore, StatusBadge } from "@job-hunter/ui";
import { Pause, Play, Square } from "lucide-react";
import { ErrorBanner, InfoBanner, PageHeader, Panel, Stat } from "../components/common";
import { ActivityFeed } from "../components/ActivityFeed";
import { useActivity } from "../lib/activity";
import { useDashboard, usePauseAgent, useProfile, useResumeAgent, useSetupStatus, useStartJobHunt, useStopJobHunt } from "../lib/queries";

export function DashboardPage() {
  const dashboard = useDashboard();
  const setup = useSetupStatus();
  const profile = useProfile();
  const start = useStartJobHunt();
  const stop = useStopJobHunt();
  const pause = usePauseAgent();
  const resume = useResumeAgent();
  const activity = useActivity();
  const navigate = useNavigate();

  const d = dashboard.data;
  const agent = d?.agent;
  const running = agent ? isAgentRunning(agent.state) : false;
  const name = profile.data?.personal.name?.split(" ")[0];
  const ready = setup.data ? (setup.data.mockMode || (setup.data.claude.authenticated && setup.data.chrome.installed)) && setup.data.profile.ready : false;

  return (
    <Box>
      <PageHeader
        title={`${greeting()}${name ? `, ${name}` : ""}`}
        subtitle={running ? `Job hunt running: ${agent?.currentActivity ?? AGENT_STATE_LABELS[agent!.state]}` : d?.lastRun ? `Last run ${timeAgo(d.lastRun.startedAt)} · ${AGENT_STATE_LABELS[d.lastRun.state]}` : "Your job hunt hasn't started yet."}
        actions={
          running ? (
            <>
              {agent?.paused ? (
                <Button variant="subtle" onClick={() => resume.mutate()} loading={resume.isPending}>
                  <Play size={16} /> Resume
                </Button>
              ) : (
                <Button variant="subtle" onClick={() => pause.mutate()} loading={pause.isPending}>
                  <Pause size={16} /> Pause
                </Button>
              )}
              <Button colorPalette="red" variant="subtle" onClick={() => stop.mutate()} loading={stop.isPending}>
                <Square size={16} /> Stop
              </Button>
            </>
          ) : (
            <Button colorPalette="brand" size="lg" onClick={() => start.mutate({})} loading={start.isPending} disabled={!ready}>
              <Play size={16} /> Start Job Hunt
            </Button>
          )
        }
      />
      <ErrorBanner error={start.error} title="Could not start the job hunt" />
      <ErrorBanner error={dashboard.error} title="Could not load the dashboard" onRetry={() => dashboard.refetch()} />
      {setup.data && !ready ? (
        <InfoBanner status="warning" title="Almost ready">
          {!setup.data.profile.ready ? "Complete your candidate profile (name, email, target roles and skills). " : ""}
          {!setup.data.mockMode && !setup.data.claude.authenticated ? "Claude CLI is not signed in. " : ""}
          {!setup.data.mockMode && !setup.data.chrome.installed ? "Chrome was not found. " : ""}
          <Link to="/setup">Open setup</Link>
        </InfoBanner>
      ) : null}
      {agent?.error ? (
        <InfoBanner status="error" title="The last run failed">
          {agent.error}
        </InfoBanner>
      ) : null}

      {!d ? (
        <Spinner />
      ) : (
        <Grid templateColumns={{ base: "1fr", xl: "2fr 1fr" }} gap={6}>
          <GridItem>
            <Heading size="sm" mb={3} color="fg.muted" textTransform="uppercase" letterSpacing="wide">
              Today&apos;s job hunt
            </Heading>
            <SimpleGrid columns={{ base: 2, md: 4 }} gap={3} mb={6}>
              <Stat label="Jobs discovered" value={d.today.jobsDiscovered} />
              <Stat label="Relevant" value={d.today.relevant} tone="blue.fg" />
              <Stat label="Awaiting approval" value={d.total.awaitingApproval} tone="purple.fg" />
              <Stat label="Applied" value={d.today.applied} tone="green.fg" />
              <Stat label="Manual action" value={d.total.manualAction + d.total.waitingForUser} tone="orange.fg" />
              <Stat label="Interviews" value={d.total.interviews} tone="green.fg" />
              <Stat label="Rejected" value={d.total.rejected} tone="red.fg" />
              <Stat label="All jobs" value={d.total.jobsDiscovered} />
            </SimpleGrid>

            {d.pendingActions.length > 0 ? (
              <Panel title={`${d.pendingActions.length} pending action${d.pendingActions.length === 1 ? "" : "s"}`} mb={6} action={<Link to="/applications">View all</Link>}>
                <VStack align="stretch" gap={2}>
                  {d.pendingActions.slice(0, 6).map((item) => (
                    <HStack key={item.application.id} justify="space-between" p={3} borderWidth="1px" borderColor="border.muted" borderRadius="md" cursor="pointer" _hover={{ bg: "bg.muted" }} onClick={() => navigate({ to: "/applications/$applicationId", params: { applicationId: item.application.id } })}>
                      <Box minW={0}>
                        <Text fontWeight="semibold" truncate>
                          {item.job.title}
                        </Text>
                        <Text fontSize="sm" color="fg.muted" truncate>
                          {item.job.company} · {item.job.location || item.job.source}
                        </Text>
                      </Box>
                      <HStack gap={3} flexShrink={0}>
                        <MatchScore score={item.analysis?.matchScore} relevant={item.analysis?.relevant} compact />
                        <StatusBadge status={item.application.status} />
                        <Button size="xs" colorPalette="brand" variant={item.application.status === "READY_FOR_REVIEW" ? "solid" : "subtle"}>
                          {item.application.status === "READY_FOR_REVIEW" ? "Review" : item.application.status === "WAITING_FOR_USER" ? "Answer" : "Continue"}
                        </Button>
                      </HStack>
                    </HStack>
                  ))}
                </VStack>
              </Panel>
            ) : null}

            <Panel title="Latest jobs" action={<Link to="/jobs">All jobs</Link>} p={0}>
              {d.latestJobs.length === 0 ? (
                <Text p={5} fontSize="sm" color="fg.muted">
                  No jobs yet. Start a job hunt to discover recent postings.
                </Text>
              ) : (
                <Table.Root size="sm" interactive>
                  <Table.Header>
                    <Table.Row>
                      <Table.ColumnHeader>Job</Table.ColumnHeader>
                      <Table.ColumnHeader>Source</Table.ColumnHeader>
                      <Table.ColumnHeader>Match</Table.ColumnHeader>
                      <Table.ColumnHeader>Status</Table.ColumnHeader>
                    </Table.Row>
                  </Table.Header>
                  <Table.Body>
                    {d.latestJobs.map((item) => (
                      <Table.Row key={item.job.id} cursor="pointer" onClick={() => navigate({ to: "/jobs/$jobId", params: { jobId: item.job.id } })}>
                        <Table.Cell>
                          <Text fontWeight="medium">{item.job.title}</Text>
                          <Text fontSize="xs" color="fg.muted">
                            {item.job.company} · {item.job.location}
                          </Text>
                        </Table.Cell>
                        <Table.Cell>{item.job.source}</Table.Cell>
                        <Table.Cell>
                          <MatchScore score={item.analysis?.matchScore} relevant={item.analysis?.relevant} compact />
                        </Table.Cell>
                        <Table.Cell>
                          <StatusBadge status={item.job.status} />
                        </Table.Cell>
                      </Table.Row>
                    ))}
                  </Table.Body>
                </Table.Root>
              )}
            </Panel>
          </GridItem>

          <GridItem>
            <Panel title="Agent activity" action={<Link to="/agent">Details</Link>} mb={6}>
              {running ? (
                <Box mb={3}>
                  <HStack justify="space-between" mb={1}>
                    <Text fontSize="sm" fontWeight="semibold">
                      {AGENT_STATE_LABELS[agent!.state]}
                    </Text>
                    <Text fontSize="xs" color="fg.muted" fontVariantNumeric="tabular-nums">
                      {agent?.progress != null ? `${agent.progress}%` : ""}
                    </Text>
                  </HStack>
                  <Box h="6px" bg="bg.muted" borderRadius="full" overflow="hidden">
                    <Box h="100%" w={`${agent?.progress ?? (agent?.paused ? 0 : 5)}%`} bg="brand.solid" transition="width 0.4s" />
                  </Box>
                  <SimpleGrid columns={3} gap={2} mt={3} fontSize="xs">
                    <Box>
                      <Text color="fg.muted">Discovered</Text>
                      <Text fontWeight="bold">{agent?.stats.jobsDiscovered}</Text>
                    </Box>
                    <Box>
                      <Text color="fg.muted">Relevant</Text>
                      <Text fontWeight="bold">{agent?.stats.relevant}</Text>
                    </Box>
                    <Box>
                      <Text color="fg.muted">Awaiting approval</Text>
                      <Text fontWeight="bold">{agent?.stats.awaitingApproval}</Text>
                    </Box>
                  </SimpleGrid>
                </Box>
              ) : null}
              <ActivityFeed events={activity.slice(-40)} maxHeight="360px" compact />
            </Panel>
            <Panel title="Recent applications" action={<Link to="/applications">All</Link>}>
              {d.recentApplications.length === 0 ? (
                <Text fontSize="sm" color="fg.muted">
                  No applications yet.
                </Text>
              ) : (
                <VStack align="stretch" gap={2}>
                  {d.recentApplications.slice(0, 6).map((item) => (
                    <HStack key={item.application.id} justify="space-between" cursor="pointer" onClick={() => navigate({ to: "/applications/$applicationId", params: { applicationId: item.application.id } })}>
                      <Box minW={0}>
                        <Text fontSize="sm" fontWeight="medium" truncate>
                          {item.job.title}
                        </Text>
                        <Text fontSize="xs" color="fg.muted" truncate>
                          {item.job.company} · {timeAgo(item.application.updatedAt)}
                        </Text>
                      </Box>
                      <StatusBadge status={item.application.status} size="xs" />
                    </HStack>
                  ))}
                </VStack>
              )}
            </Panel>
          </GridItem>
        </Grid>
      )}
    </Box>
  );
}
