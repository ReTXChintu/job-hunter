import { Box, Button, Circle, Flex, HStack, IconButton, Link, Popover, Portal, Text, VStack } from "@chakra-ui/react";
import { useQuery } from "@tanstack/react-query";
import { formatDateTime } from "@job-hunter/shared";
import type { Notification } from "@job-hunter/types";
import { Bell } from "lucide-react";
import { useEffect, useState } from "react";

import { markAllSeen, notificationHref, seenAt, sortNewest, takeFresh, unreadCount } from "../notifications";
import { useSession } from "../session";

const LEVEL_COLOR: Record<Notification["level"], string> = {
  INFO: "blue.solid",
  SUCCESS: "green.solid",
  WARN: "orange.solid",
  ERROR: "red.solid",
};

const canNotify = () => typeof window !== "undefined" && "Notification" in window;

/** Browser alerts for notifications that arrive while this tab is open. */
function alert(n: Notification) {
  if (!canNotify() || window.Notification.permission !== "granted") return;
  const shown = new window.Notification(n.title, { body: n.body, tag: n.id });
  const target = notificationHref(n);
  shown.onclick = () => {
    window.focus();
    if (target) window.location.hash = target;
  };
}

export function NotificationBell() {
  const { api } = useSession();
  const [open, setOpen] = useState(false);
  const [seen, setSeen] = useState(seenAt);
  const [permission, setPermission] = useState(() => (canNotify() ? window.Notification.permission : "denied"));
  const query = useQuery({ queryKey: ["notifications"], queryFn: async () => sortNewest(await api.notifications()), refetchInterval: 30_000 });
  const items = query.data ?? [];
  const unread = unreadCount(items, seen);

  useEffect(() => {
    if (query.data) for (const n of takeFresh(query.data)) alert(n);
  }, [query.data]);

  const onOpenChange = (next: boolean) => {
    setOpen(next);
    // Closing the list marks what was in it as seen.
    if (!next && items[0]) {
      markAllSeen(items[0].createdAt);
      setSeen(items[0].createdAt);
    }
  };

  return (
    <Popover.Root open={open} onOpenChange={(e) => onOpenChange(e.open)} positioning={{ placement: "bottom-end" }}>
      <Popover.Trigger asChild>
        <Box position="relative">
          <IconButton aria-label={unread ? `${unread} new notifications` : "Notifications"} variant="ghost" size="sm">
            <Bell size={16} />
          </IconButton>
          {unread ? (
            <Flex position="absolute" top="0" right="0" minW={4} h={4} px={1} borderRadius="full" bg="red.solid" color="white" fontSize="2xs" fontWeight="bold" align="center" justify="center" pointerEvents="none">
              {unread > 9 ? "9+" : unread}
            </Flex>
          ) : null}
        </Box>
      </Popover.Trigger>
      <Portal>
        <Popover.Positioner>
          <Popover.Content width={{ base: "calc(100vw - 32px)", sm: "360px" }}>
            <Popover.Body p={0}>
              <HStack justify="space-between" px={4} py={2.5} borderBottomWidth="1px" borderColor="border.muted">
                <Text fontWeight="semibold" fontSize="sm">
                  Notifications
                </Text>
                {permission === "default" ? (
                  <Button size="2xs" variant="subtle" onClick={() => void window.Notification.requestPermission().then(setPermission)}>
                    Alert me in this browser
                  </Button>
                ) : null}
              </HStack>
              {items.length === 0 ? (
                <Text fontSize="sm" color="fg.muted" px={4} py={6} textAlign="center">
                  Nothing yet. Finished job hunts, applications that need you and failures show up here.
                </Text>
              ) : (
                <VStack align="stretch" gap={0} maxH="420px" overflowY="auto">
                  {items.map((n) => {
                    const target = notificationHref(n);
                    const body = (
                      <HStack align="flex-start" gap={3} px={4} py={2.5} bg={n.createdAt > seen ? "bg.muted" : undefined} borderBottomWidth="1px" borderColor="border.muted">
                        <Circle size={2} mt={1.5} flexShrink={0} bg={LEVEL_COLOR[n.level]} />
                        <Box minW={0}>
                          <Text fontSize="sm" fontWeight={n.createdAt > seen ? "semibold" : "normal"}>
                            {n.title}
                          </Text>
                          {n.body ? (
                            <Text fontSize="xs" color="fg.muted" lineClamp={2}>
                              {n.body}
                            </Text>
                          ) : null}
                          <Text fontSize="2xs" color="fg.subtle" mt={0.5}>
                            {formatDateTime(n.createdAt)}
                            {target ? "" : " · open the desktop app for this"}
                          </Text>
                        </Box>
                      </HStack>
                    );
                    return target ? (
                      <Link key={n.id} href={target} display="block" _hover={{ textDecoration: "none", bg: "bg.emphasized" }} onClick={() => onOpenChange(false)}>
                        {body}
                      </Link>
                    ) : (
                      <Box key={n.id}>{body}</Box>
                    );
                  })}
                </VStack>
              )}
            </Popover.Body>
          </Popover.Content>
        </Popover.Positioner>
      </Portal>
    </Popover.Root>
  );
}
