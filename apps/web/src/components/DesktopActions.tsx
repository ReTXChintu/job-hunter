import { Alert, Button, Flex, Text, VStack } from "@chakra-ui/react";
import { AGENT_STATE_LABELS, isAgentRunning } from "@job-hunter/shared";
import { useQuery } from "@tanstack/react-query";
import { Mail, Megaphone, Play, Square } from "lucide-react";

import { useDesktopAction } from "../desktopActions";
import { useSession } from "../session";
import { ErrorBox, Panel } from "./State";

/**
 * Starts and stops work on the desktop from the web. The agent's state is
 * asked of the desktop itself (only while it's online), so the buttons match
 * what it's doing; if that's unknown, every button stays available and the
 * desktop's reply says whether it worked.
 */
export function DesktopActions({ desktopOnline }: { desktopOnline: boolean | null }) {
  const { api } = useSession();
  const status = useQuery({
    queryKey: ["agent-status"],
    queryFn: () => api.agentStatus(),
    enabled: desktopOnline === true,
    refetchInterval: 15_000,
    retry: false,
  });
  const action = useDesktopAction([["agent-status"], ["notifications"], ["applications"], ["jobs"]]);

  const known = desktopOnline === true && status.isSuccess;
  const running = known && isAgentRunning(status.data.state);
  const progress = running && status.data.progress != null ? ` · ${Math.round(status.data.progress)}%` : "";

  const start = (key: string, sources: string[] | undefined, success: string) =>
    action.run({ key, run: () => api.startJobHunt(sources ? { sources } : {}), success });

  return (
    <Panel title="On your desktop">
      <VStack align="stretch" gap={3}>
        <Text fontSize="sm" color="fg.muted" aria-live="polite">
          {desktopOnline === false
            ? "Your desktop is offline. Open Job Hunter on your computer to run these."
            : known
              ? `${AGENT_STATE_LABELS[status.data.state] ?? status.data.state}${progress}`
              : "Runs on your computer, the same as its own buttons."}
        </Text>
        <Flex gap={2} wrap="wrap">
          <Button
            size="sm"
            colorPalette="brand"
            onClick={() => start("hunt", undefined, "Job hunt started on your desktop. New matches show up here as they're found.")}
            loading={action.running("hunt")}
            loadingText="Starting…"
            disabled={action.busy || running}
          >
            <Play size={14} /> Start job hunt
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => start("posts", ["LinkedIn Posts"], "Looking for hiring posts on LinkedIn from your desktop.")}
            loading={action.running("posts")}
            loadingText="Starting…"
            disabled={action.busy || running}
          >
            <Megaphone size={14} /> Find hiring posts
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => action.run({ key: "inbox", run: () => api.checkInbox(), success: "Your desktop is checking Gmail for replies." })}
            loading={action.running("inbox")}
            loadingText="Starting…"
            disabled={action.busy || running}
          >
            <Mail size={14} /> Check Gmail for replies
          </Button>
          <Button
            size="sm"
            variant="outline"
            colorPalette="red"
            onClick={() =>
              action.run({
                key: "stop",
                run: async () => {
                  const { stopped } = await api.stopJobHunt();
                  if (!stopped) throw new Error("Nothing was running on your desktop.");
                },
                success: "Stopping. Your desktop finishes the current step first.",
              })
            }
            loading={action.running("stop")}
            loadingText="Stopping…"
            disabled={action.busy || (known && !running)}
          >
            <Square size={14} /> Stop
          </Button>
        </Flex>
        {action.success ? (
          <Alert.Root status="success" borderRadius="md" role="status" aria-live="polite">
            <Alert.Indicator />
            <Alert.Content>
              <Alert.Description>{action.success}</Alert.Description>
            </Alert.Content>
          </Alert.Root>
        ) : null}
        {action.error ? <ErrorBox error={action.error} title="Your desktop couldn't do that" /> : null}
      </VStack>
    </Panel>
  );
}
