import { Box, HStack, Text, VStack } from "@chakra-ui/react";
import type { AgentEvent } from "@job-hunter/types";
import { AlertTriangle, Check, Circle, XCircle } from "lucide-react";
import { useEffect, useRef } from "react";

function Icon({ ev }: { ev: AgentEvent }) {
  if (ev.level === "ERROR") return <XCircle size={14} />;
  if (ev.level === "WARN") return <AlertTriangle size={14} />;
  if (ev.level === "SUCCESS" || ev.kind === "STEP_DONE") return <Check size={14} />;
  return <Circle size={8} style={{ marginTop: 3 }} />;
}

function color(ev: AgentEvent): string {
  if (ev.level === "ERROR") return "red.fg";
  if (ev.level === "WARN") return "orange.fg";
  if (ev.level === "SUCCESS" || ev.kind === "STEP_DONE") return "green.fg";
  return "fg.muted";
}

export function ActivityFeed({ events, maxHeight = "420px", compact = false, autoScroll = true }: { events: AgentEvent[]; maxHeight?: string; compact?: boolean; autoScroll?: boolean }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (autoScroll && ref.current) ref.current.scrollTop = ref.current.scrollHeight;
  }, [events.length, autoScroll]);
  if (!events.length) {
    return (
      <Text fontSize="sm" color="fg.subtle" py={4}>
        No activity yet.
      </Text>
    );
  }
  return (
    <Box ref={ref} maxH={maxHeight} overflowY="auto" pr={2}>
      <VStack align="stretch" gap={compact ? 1 : 1.5}>
        {events.map((ev) => (
          <HStack key={ev.id} align="flex-start" gap={2.5} fontSize={compact ? "xs" : "sm"} opacity={ev.kind === "CLAUDE_ACTIVITY" ? 0.85 : 1}>
            <Box color={color(ev)} pt="2px" flexShrink={0} w="14px" display="flex" justifyContent="center">
              <Icon ev={ev} />
            </Box>
            <Text flex="1" className="selectable" whiteSpace="pre-wrap" color={ev.kind === "CLAUDE_ACTIVITY" ? "fg.muted" : "fg"}>
              {ev.message}
            </Text>
            <Text color="fg.subtle" fontVariantNumeric="tabular-nums" flexShrink={0} fontSize="xs">
              {new Date(ev.at).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit", second: "2-digit" })}
            </Text>
          </HStack>
        ))}
      </VStack>
    </Box>
  );
}
