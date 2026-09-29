import { Badge, Box, HStack, Text, VStack } from "@chakra-ui/react";
import { formatDateTime } from "@job-hunter/shared";
import type { EmailReply } from "@job-hunter/types";

import { Panel } from "./State";

const KIND_PALETTE: Record<string, string> = {
  INTERVIEW: "green",
  OFFER: "green",
  ASSESSMENT: "orange",
  QUESTION: "orange",
  REJECTION: "red",
  ACKNOWLEDGEMENT: "blue",
};

const kindLabel = (kind: string) => kind.charAt(0) + kind.slice(1).toLowerCase();

/** Employers' replies the desktop found in Gmail (inbox and spam), newest first. */
export function Replies({ replies, company }: { replies: EmailReply[]; company: string }) {
  const sorted = [...replies].sort((a, b) => (b.receivedAt || b.foundAt).localeCompare(a.receivedAt || a.foundAt));
  const inSpam = replies.some((r) => r.folder === "SPAM");
  return (
    <Panel title={`Replies from ${company}`}>
      <VStack align="stretch" gap={3}>
        {sorted.map((r) => (
          <Box key={r.id} as="article" p={3} borderWidth="1px" borderColor="border.muted" borderRadius="md">
            <HStack gap={2} mb={1} wrap="wrap">
              <Badge colorPalette={KIND_PALETTE[r.kind] ?? "gray"} variant="subtle">
                {kindLabel(r.kind)}
              </Badge>
              {r.folder === "SPAM" ? (
                <Badge colorPalette="red" variant="solid">
                  In Spam
                </Badge>
              ) : null}
              <Text fontSize="sm" fontWeight="semibold">
                {r.subject || "(no subject)"}
              </Text>
            </HStack>
            {r.summary ? <Text fontSize="sm">{r.summary}</Text> : null}
            <Text fontSize="xs" color="fg.muted" mt={1}>
              {r.from}
              {r.receivedAt ? ` · ${formatDateTime(r.receivedAt)}` : ""}
            </Text>
          </Box>
        ))}
      </VStack>
      <Text fontSize="xs" color="fg.subtle" mt={3}>
        Found in your Gmail by the desktop, which only reads it.
        {inSpam ? " Move anything found in Spam to your inbox so you don't miss follow-ups." : ""}
      </Text>
    </Panel>
  );
}
