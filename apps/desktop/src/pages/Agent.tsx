import { Box, Button, Grid, GridItem, HStack, NativeSelect, SimpleGrid, Table, Tabs, Text, VStack } from "@chakra-ui/react";
import { AGENT_STATE_LABELS, formatDateTime, isAgentRunning } from "@job-hunter/shared";
import type { LogLevel } from "@job-hunter/types";
import { Copy, Download, Pause, Play, Square, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import { ErrorBanner, PageHeader, Panel, Stat } from "../components/common";
import { ActivityFeed } from "../components/ActivityFeed";
import { useActivity, useLogFeed } from "../lib/activity";
import { useAgentStatus, useClearLogs, useExportLogs, useLogs, usePauseAgent, useResumeAgent, useRunEvents, useRuns, useStartJobHunt, useStopJobHunt } from "../lib/queries";

export function AgentPage() {
  const status = useAgentStatus();
  const runs = useRuns(30);
  const start = useStartJobHunt();
  const stop = useStopJobHunt();
  const pause = usePauseAgent();
  const resume = useResumeAgent();
  const activity = useActivity();
  const [selectedRun, setSelectedRun] = useState<string | null>(null);
  const runEvents = useRunEvents(selectedRun);
  const s = status.data;
  const running = s ? isAgentRunning(s.state) : false;
  const liveEvents = useMemo(() => (s?.runId ? activity.filter((e) => e.runId === s.runId) : activity), [activity, s?.runId]);

  return (
    <Box>
      <PageHeader
        title="Agent"
        subtitle={s ? `${s.paused ? "Paused" : AGENT_STATE_LABELS[s.state]}${s.currentActivity ? ` · ${s.currentActivity}` : ""}` : ""}
        actions={
          running ? (
            <>
              {s?.paused ? (
                <Button variant="subtle" onClick={() => resume.mutate()} loading={resume.isPending}><Play size={16} /> Resume</Button>
              ) : (
                <Button variant="subtle" onClick={() => pause.mutate()} loading={pause.isPending}><Pause size={16} /> Pause</Button>
              )}
              <Button colorPalette="red" variant="subtle" onClick={() => stop.mutate()} loading={stop.isPending}><Square size={16} /> Stop</Button>
            </>
          ) : (
            <Button colorPalette="brand" onClick={() => start.mutate({})} loading={start.isPending}><Play size={16} /> Start Job Hunt</Button>
          )
        }
      />
      <ErrorBanner error={start.error ?? stop.error ?? pause.error ?? resume.error} />
      <SimpleGrid columns={{ base: 2, md: 6 }} gap={3} mb={6}>
        <Stat label="Discovered" value={s?.stats.jobsDiscovered ?? 0} />
        <Stat label="New" value={s?.stats.jobsNew ?? 0} />
        <Stat label="Relevant" value={s?.stats.relevant ?? 0} tone="blue.fg" />
        <Stat label="Awaiting approval" value={s?.stats.awaitingApproval ?? 0} tone="purple.fg" />
        <Stat label="Errors" value={s?.stats.errors ?? 0} tone={s?.stats.errors ? "red.fg" : "fg"} />
        <Stat label="Claude cost" value={`$${(s?.stats.claudeCostUsd ?? 0).toFixed(2)}`} />
      </SimpleGrid>

      <Tabs.Root defaultValue="activity" variant="line" size="sm">
        <Tabs.List mb={4}>
          <Tabs.Trigger value="activity">Live activity</Tabs.Trigger>
          <Tabs.Trigger value="runs">Runs</Tabs.Trigger>
          <Tabs.Trigger value="logs">Logs</Tabs.Trigger>
        </Tabs.List>
        <Tabs.Content value="activity">
          <Panel>
            {running && s?.progress != null ? (
              <Box mb={4}>
                <HStack justify="space-between" mb={1} fontSize="sm">
                  <Text fontWeight="semibold">{AGENT_STATE_LABELS[s.state]}</Text>
                  <Text fontVariantNumeric="tabular-nums">{s.progress}%</Text>
                </HStack>
                <Box h="8px" bg="bg.muted" borderRadius="full" overflow="hidden">
                  <Box h="100%" w={`${s.progress}%`} bg="brand.solid" transition="width 0.3s" />
                </Box>
              </Box>
            ) : null}
            <ActivityFeed events={liveEvents} maxHeight="60vh" />
          </Panel>
        </Tabs.Content>
        <Tabs.Content value="runs">
          <Grid templateColumns={{ base: "1fr", lg: "1fr 1fr" }} gap={4}>
            <GridItem>
              <Panel p={0}>
                <Table.Root size="sm" interactive>
                  <Table.Header>
                    <Table.Row>
                      <Table.ColumnHeader>Started</Table.ColumnHeader>
                      <Table.ColumnHeader>Kind</Table.ColumnHeader>
                      <Table.ColumnHeader>State</Table.ColumnHeader>
                      <Table.ColumnHeader>Jobs</Table.ColumnHeader>
                      <Table.ColumnHeader>Cost</Table.ColumnHeader>
                    </Table.Row>
                  </Table.Header>
                  <Table.Body>
                    {(runs.data ?? []).map((r) => (
                      <Table.Row key={r.id} cursor="pointer" bg={selectedRun === r.id ? "bg.muted" : undefined} onClick={() => setSelectedRun(r.id)}>
                        <Table.Cell>
                          <Text fontSize="sm">{formatDateTime(r.startedAt)}</Text>
                          {r.mock ? <Text fontSize="xs" color="orange.fg">mock</Text> : null}
                        </Table.Cell>
                        <Table.Cell><Text fontSize="sm">{r.kind.replace("_", " ").toLowerCase()}</Text></Table.Cell>
                        <Table.Cell>
                          <Text fontSize="sm" color={r.state === "FAILED" ? "red.fg" : undefined}>{AGENT_STATE_LABELS[r.state]}</Text>
                          {r.error ? <Text fontSize="xs" color="red.fg" maxW="240px" truncate>{r.error}</Text> : null}
                        </Table.Cell>
                        <Table.Cell><Text fontSize="sm">{r.stats.jobsDiscovered} / {r.stats.relevant} relevant</Text></Table.Cell>
                        <Table.Cell><Text fontSize="sm">${r.stats.claudeCostUsd.toFixed(2)}</Text></Table.Cell>
                      </Table.Row>
                    ))}
                  </Table.Body>
                </Table.Root>
                {runs.data?.length === 0 ? <Text p={4} fontSize="sm" color="fg.muted">No runs yet.</Text> : null}
              </Panel>
            </GridItem>
            <GridItem>
              <Panel title={selectedRun ? "Run events" : "Select a run"}>
                <ActivityFeed events={runEvents.data ?? []} maxHeight="60vh" compact autoScroll={false} />
              </Panel>
            </GridItem>
          </Grid>
        </Tabs.Content>
        <Tabs.Content value="logs">
          <LogViewer />
        </Tabs.Content>
      </Tabs.Root>
    </Box>
  );
}

const LEVEL_ORDER: LogLevel[] = ["DEBUG", "INFO", "WARN", "ERROR"];

export function LogViewer() {
  const [level, setLevel] = useState<LogLevel>("INFO");
  const logs = useLogs(level);
  const live = useLogFeed();
  const clear = useClearLogs();
  const exportLogs = useExportLogs();
  const [copied, setCopied] = useState(false);
  const entries = useMemo(() => {
    const seen = new Set<string>();
    const all = [...(logs.data ?? []), ...live].filter((e) => LEVEL_ORDER.indexOf(e.level) >= LEVEL_ORDER.indexOf(level));
    return all.filter((e) => (seen.has(e.id) ? false : (seen.add(e.id), true))).slice(-1500);
  }, [logs.data, live, level]);
  const copy = async () => {
    await navigator.clipboard.writeText(entries.map((e) => `${e.at} ${e.level} [${e.target}] ${e.message}`).join("\n"));
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };
  return (
    <Panel
      title="Logs"
      action={
        <HStack gap={2}>
          <NativeSelect.Root size="xs" w="110px">
            <NativeSelect.Field value={level} onChange={(e) => setLevel(e.target.value as LogLevel)}>
              {LEVEL_ORDER.map((l) => <option key={l} value={l}>{l}</option>)}
            </NativeSelect.Field>
            <NativeSelect.Indicator />
          </NativeSelect.Root>
          <Button size="xs" variant="ghost" onClick={copy}><Copy size={12} /> {copied ? "Copied" : "Copy Logs"}</Button>
          <Button size="xs" variant="ghost" onClick={() => exportLogs.mutate()} loading={exportLogs.isPending}><Download size={12} /> Export Logs</Button>
          <Button size="xs" variant="ghost" colorPalette="red" onClick={() => clear.mutate()}><Trash2 size={12} /> Clear Logs</Button>
        </HStack>
      }
    >
      {exportLogs.data ? <Text fontSize="xs" color="green.fg" mb={2}>Exported to {exportLogs.data}</Text> : null}
      <Box fontFamily="mono" fontSize="xs" maxH="60vh" overflowY="auto" className="selectable">
        <VStack align="stretch" gap={0}>
          {entries.map((e) => (
            <HStack key={e.id} gap={3} py={0.5} borderBottomWidth="1px" borderColor="border.muted" align="flex-start">
              <Text color="fg.subtle" flexShrink={0}>{new Date(e.at).toLocaleTimeString()}</Text>
              <Text flexShrink={0} w="44px" fontWeight="bold" color={e.level === "ERROR" ? "red.fg" : e.level === "WARN" ? "orange.fg" : e.level === "DEBUG" ? "fg.subtle" : "fg.muted"}>{e.level}</Text>
              <Text color="fg.muted" flexShrink={0} maxW="160px" truncate>{e.target}</Text>
              <Text whiteSpace="pre-wrap">{e.message}</Text>
            </HStack>
          ))}
          {entries.length === 0 ? <Text color="fg.muted">No log entries at this level.</Text> : null}
        </VStack>
      </Box>
    </Panel>
  );
}
