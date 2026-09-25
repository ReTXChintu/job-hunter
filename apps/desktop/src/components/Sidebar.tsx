import { Box, Flex, HStack, IconButton, Text, VStack } from "@chakra-ui/react";
import { Link, useRouterState } from "@tanstack/react-router";
import { useTheme } from "next-themes";
import { Bot, Briefcase, FileText, LayoutDashboard, Moon, Send, Settings, Sun, UserRound, Crosshair } from "lucide-react";
import type { ReactNode } from "react";
import { useAboutInfo, useDashboard, useUpdateCheck } from "../lib/queries";

const NAV: { to: string; label: string; icon: ReactNode; badge?: (n: { awaiting: number; manual: number }) => number }[] = [
  { to: "/", label: "Dashboard", icon: <LayoutDashboard size={18} /> },
  { to: "/jobs", label: "Jobs", icon: <Briefcase size={18} /> },
  { to: "/applications", label: "Applications", icon: <Send size={18} />, badge: (n) => n.awaiting + n.manual },
  { to: "/candidate", label: "Candidate", icon: <UserRound size={18} /> },
  { to: "/resumes", label: "Resumes", icon: <FileText size={18} /> },
  { to: "/agent", label: "Agent", icon: <Bot size={18} /> },
  { to: "/settings", label: "Settings", icon: <Settings size={18} /> },
];

export function Sidebar() {
  const path = useRouterState({ select: (s) => s.location.pathname });
  const { resolvedTheme, setTheme } = useTheme();
  const dashboard = useDashboard();
  const about = useAboutInfo();
  const update = useUpdateCheck();
  const counts = {
    awaiting: dashboard.data?.total.awaitingApproval ?? 0,
    manual: (dashboard.data?.total.manualAction ?? 0) + (dashboard.data?.total.waitingForUser ?? 0),
  };

  return (
    <Flex direction="column" w="232px" flexShrink={0} bg="bg.sidebar" borderRightWidth="1px" borderColor="border.muted" px={3} py={4}>
      <HStack px={3} pb={5} gap={2.5}>
        <Flex w={8} h={8} align="center" justify="center" borderRadius="md" bg="brand.solid" color="brand.contrast">
          <Crosshair size={18} />
        </Flex>
        <Box>
          <Text fontWeight="bold" lineHeight="1.1">
            Job Hunter
          </Text>
          <Text fontSize="xs" color="fg.muted">
            Command center
          </Text>
        </Box>
      </HStack>
      <VStack align="stretch" gap={0.5} flex="1">
        {NAV.map((item) => {
          const active = item.to === "/" ? path === "/" : path.startsWith(item.to);
          const badge = item.badge?.(counts) ?? 0;
          return (
            <Link key={item.to} to={item.to} style={{ textDecoration: "none" }}>
              <HStack
                px={3}
                py={2}
                borderRadius="md"
                gap={3}
                bg={active ? "bg.emphasized" : "transparent"}
                color={active ? "fg" : "fg.muted"}
                fontWeight={active ? "semibold" : "medium"}
                _hover={{ bg: active ? "bg.emphasized" : "bg.muted", color: "fg" }}
                transition="background 0.12s"
              >
                <Box color={active ? "brand.fg" : "inherit"}>{item.icon}</Box>
                <Text flex="1" fontSize="sm">
                  {item.label}
                </Text>
                {badge > 0 ? (
                  <Box fontSize="xs" fontWeight="bold" px={1.5} minW={5} textAlign="center" borderRadius="full" bg="brand.solid" color="brand.contrast">
                    {badge}
                  </Box>
                ) : null}
              </HStack>
            </Link>
          );
        })}
      </VStack>
      <HStack px={2} pt={3} justify="space-between">
<Link to="/about">
          <HStack gap={1.5} fontSize="xs" color="fg.subtle" _hover={{ color: "fg" }} title="About Job Hunter">
            <Text>{about.data ? `v${about.data.version}` : "About"}</Text>
            {update.data?.available ? <Box w={1.5} h={1.5} borderRadius="full" bg="brand.solid" title="Update available" /> : null}
          </HStack>
        </Link>
        <IconButton aria-label="Toggle color mode" size="xs" variant="ghost" onClick={() => setTheme(resolvedTheme === "dark" ? "light" : "dark")}>
          {resolvedTheme === "dark" ? <Sun size={16} /> : <Moon size={16} />}
        </IconButton>
      </HStack>
    </Flex>
  );
}
