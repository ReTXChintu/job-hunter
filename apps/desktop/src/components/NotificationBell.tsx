import { Box, Button, Circle, Flex, HStack, IconButton, Popover, Portal, Text, VStack } from "@chakra-ui/react";
import { useNavigate } from "@tanstack/react-router";
import { formatDateTime } from "@job-hunter/shared";
import type { Notification } from "@job-hunter/types";
import { Bell } from "lucide-react";
import { useState } from "react";
import { useMarkNotificationsRead, useNotifications } from "../lib/queries";

const LEVEL_COLOR: Record<Notification["level"], string> = {
  INFO: "blue.solid",
  SUCCESS: "green.solid",
  WARN: "orange.solid",
  ERROR: "red.solid",
};

/** Where a notification leads in the desktop app. */
export function notificationTarget(n: Pick<Notification, "linkPage" | "linkId">): string {
  switch (n.linkPage) {
    case "application":
      return n.linkId ? `/applications/${n.linkId}` : "/applications";
    case "applications":
      return "/applications";
    case "job-sites":
      return "/job-sites";
    case "jobs":
      return "/jobs";
    default:
      return "/agent";
  }
}

/** What the agent wants you to know: finished hunts, applications and profile updates that need you, failures. */
export function NotificationBell() {
  const list = useNotifications();
  const markRead = useMarkNotificationsRead();
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const items = list.data ?? [];
  const unread = items.filter((n) => !n.read).length;

  const openItem = (n: Notification) => {
    if (!n.read) markRead.mutate([n.id]);
    setOpen(false);
    void navigate({ to: notificationTarget(n) });
  };

  return (
    <Popover.Root open={open} onOpenChange={(e) => setOpen(e.open)} positioning={{ placement: "right-end" }}>
      <Popover.Trigger asChild>
        <Box position="relative">
          <IconButton aria-label={unread ? `${unread} unread notifications` : "Notifications"} size="xs" variant="ghost">
            <Bell size={16} />
          </IconButton>
          {unread ? (
            <Flex position="absolute" top="-1" right="-1" minW={4} h={4} px={1} borderRadius="full" bg="red.solid" color="white" fontSize="2xs" fontWeight="bold" align="center" justify="center" pointerEvents="none">
              {unread > 9 ? "9+" : unread}
            </Flex>
          ) : null}
        </Box>
      </Popover.Trigger>
      <Portal>
        <Popover.Positioner>
          <Popover.Content width="360px">
            <Popover.Body p={0}>
              <HStack justify="space-between" px={4} py={2.5} borderBottomWidth="1px" borderColor="border.muted">
                <Text fontWeight="semibold" fontSize="sm">
                  Notifications
                </Text>
                {unread ? (
                  <Button size="2xs" variant="ghost" onClick={() => markRead.mutate(undefined)} loading={markRead.isPending}>
                    Mark all read
                  </Button>
                ) : null}
              </HStack>
              {items.length === 0 ? (
                <Text fontSize="sm" color="fg.muted" px={4} py={6} textAlign="center">
                  Nothing yet. You'll hear here, in Windows, on your phone and on the web when a job hunt finishes or something needs you.
                </Text>
              ) : (
                <VStack align="stretch" gap={0} maxH="420px" overflowY="auto">
                  {items.map((n) => (
                    <HStack
                      key={n.id}
                      as="button"
                      align="flex-start"
                      gap={3}
                      px={4}
                      py={2.5}
                      textAlign="left"
                      bg={n.read ? "transparent" : "bg.muted"}
                      _hover={{ bg: "bg.emphasized" }}
                      borderBottomWidth="1px"
                      borderColor="border.muted"
                      onClick={() => openItem(n)}
                    >
                      <Circle size={2} mt={1.5} flexShrink={0} bg={LEVEL_COLOR[n.level]} />
                      <Box minW={0}>
                        <Text fontSize="sm" fontWeight={n.read ? "normal" : "semibold"}>
                          {n.title}
                        </Text>
                        {n.body ? (
                          <Text fontSize="xs" color="fg.muted" lineClamp={2}>
                            {n.body}
                          </Text>
                        ) : null}
                        <Text fontSize="2xs" color="fg.subtle" mt={0.5}>
                          {formatDateTime(n.createdAt)}
                        </Text>
                      </Box>
                    </HStack>
                  ))}
                </VStack>
              )}
            </Popover.Body>
          </Popover.Content>
        </Popover.Positioner>
      </Portal>
    </Popover.Root>
  );
}
