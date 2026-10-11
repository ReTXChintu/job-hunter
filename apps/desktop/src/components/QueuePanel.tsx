import { Button, HStack, Text, VStack } from "@chakra-ui/react";
import { timeAgo } from "@job-hunter/shared";
import { ListOrdered, X } from "lucide-react";
import { Panel } from "./common";
import { useClearQueue, useQueue, useRemoveQueuedTask } from "../lib/queries";

/** What's waiting for the agent, in the order it will run. */
export function QueuePanel() {
  const queue = useQueue();
  const remove = useRemoveQueuedTask();
  const clear = useClearQueue();
  const items = queue.data ?? [];
  return (
    <Panel
      title={`Queue${items.length ? ` (${items.length})` : ""}`}
      action={
        items.length > 1 ? (
          <Button size="xs" variant="ghost" onClick={() => clear.mutate()} loading={clear.isPending}>
            Clear all
          </Button>
        ) : undefined
      }
    >
      {items.length === 0 ? (
        <Text fontSize="sm" color="fg.muted">
          Nothing waiting. Start anything while the agent is busy and it lines up here, then runs by itself as soon as the agent is free.
        </Text>
      ) : (
        <VStack align="stretch" gap={1}>
          {items.map((item, i) => (
            <HStack key={item.id} justify="space-between" py={1.5} borderTopWidth={i ? "1px" : 0} borderColor="border.muted">
              <HStack gap={3} minW={0}>
                <ListOrdered size={14} />
                <Text fontSize="sm" fontWeight="medium" truncate>
                  {i + 1}. {item.title}
                </Text>
                <Text fontSize="xs" color="fg.muted" flexShrink={0}>
                  added {timeAgo(item.requestedAt)}
                </Text>
              </HStack>
              <Button size="2xs" variant="ghost" onClick={() => remove.mutate(item.id)} loading={remove.isPending && remove.variables === item.id} title="Remove from the queue">
                <X size={12} />
              </Button>
            </HStack>
          ))}
        </VStack>
      )}
    </Panel>
  );
}
