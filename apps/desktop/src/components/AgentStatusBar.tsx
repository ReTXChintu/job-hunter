import { Box, Button, Flex, HStack, Text } from "@chakra-ui/react";
import { Link } from "@tanstack/react-router";
import { AGENT_STATE_LABELS, isAgentRunning } from "@job-hunter/shared";
import { Pause, Play, Square } from "lucide-react";
import { useAgentStatus, usePauseAgent, useResumeAgent, useStopJobHunt, useSyncStatus } from "../lib/queries";
import { useActivity } from "../lib/activity";

export function AgentStatusBar() {
  const status = useAgentStatus();
  const sync = useSyncStatus();
  const stop = useStopJobHunt();
  const pause = usePauseAgent();
  const resume = useResumeAgent();
  const activity = useActivity();
  const s = status.data;
  const running = s ? isAgentRunning(s.state) : false;
  const last = activity[activity.length - 1];
  const dot = !s ? "gray" : s.state === "FAILED" ? "red" : running ? (s.paused ? "orange" : "green") : s.state === "WAITING_FOR_APPROVAL" || s.state === "WAITING_FOR_USER" || s.state === "MANUAL_ACTION_REQUIRED" ? "orange" : "gray";

  return (
    <Flex h="40px" px={4} align="center" gap={4} borderTopWidth="1px" borderColor="border.muted" bg="bg.sidebar" fontSize="xs">
      <HStack gap={2} minW="200px">
        <Box w={2} h={2} borderRadius="full" bg={`${dot}.solid`} boxShadow={running && !s?.paused ? `0 0 0 3px var(--chakra-colors-${dot}-muted)` : undefined} />
        <Text fontWeight="semibold">{s ? (s.paused ? "Paused" : AGENT_STATE_LABELS[s.state]) : "Connecting"}</Text>
        {s?.mock ? (
          <Text px={1.5} borderRadius="sm" bg="orange.muted" color="orange.fg" fontWeight="bold">
            MOCK
          </Text>
        ) : null}
      </HStack>
      <Box flex="1" minW={0} color="fg.muted" truncate>
        {running ? (s?.currentActivity ?? last?.message ?? "") : s?.error ? s.error : last?.message ?? ""}
      </Box>
      {running && s?.progress != null ? (
        <HStack gap={2} minW="140px">
          <Box flex="1" h="4px" bg="bg.muted" borderRadius="full" overflow="hidden">
            <Box h="100%" w={`${s.progress}%`} bg="brand.solid" transition="width 0.3s" />
          </Box>
          <Text fontVariantNumeric="tabular-nums">{s.progress}%</Text>
        </HStack>
      ) : null}
      {running ? (
        <HStack gap={1}>
          {s?.paused ? (
            <Button size="2xs" variant="subtle" onClick={() => resume.mutate()} loading={resume.isPending}>
              <Play size={12} /> Resume
            </Button>
          ) : (
            <Button size="2xs" variant="subtle" onClick={() => pause.mutate()} loading={pause.isPending} disabled={s?.state === "STOPPING"}>
              <Pause size={12} /> Pause
            </Button>
          )}
          <Button size="2xs" variant="subtle" colorPalette="red" onClick={() => stop.mutate()} loading={stop.isPending} disabled={s?.state === "STOPPING"}>
            <Square size={12} /> Stop
          </Button>
        </HStack>
      ) : null}
      <Link to="/settings">
        <HStack gap={1.5} color={sync.data?.connected ? "green.fg" : sync.data?.configured ? "orange.fg" : "fg.subtle"}>
          <Box w={1.5} h={1.5} borderRadius="full" bg="currentColor" />
          <Text>{sync.data?.connected ? "Atlas synced" : sync.data?.configured ? `Atlas offline · ${sync.data.pending} pending` : "Local only"}</Text>
        </HStack>
      </Link>
    </Flex>
  );
}
