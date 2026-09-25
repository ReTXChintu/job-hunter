import { Box, Button, Container, Flex, HStack, IconButton, Link, Text } from "@chakra-ui/react";
import { Briefcase, LogOut, Moon, Sun } from "lucide-react";
import { useTheme } from "next-themes";
import type { ReactNode } from "react";

import { href, type Route } from "../route";
import { useSession } from "../session";

const NAV: { label: string; route: Route; matches: Route["page"][] }[] = [
  { label: "Overview", route: { page: "overview" }, matches: ["overview"] },
  { label: "Applications", route: { page: "applications" }, matches: ["applications", "application"] },
  { label: "Jobs", route: { page: "jobs" }, matches: ["jobs"] },
];

export function Layout({ route, children }: { route: Route; children: ReactNode }) {
  const { api, session } = useSession();
  const { resolvedTheme, setTheme } = useTheme();
  return (
    <Box minH="100vh">
      <Box as="header" borderBottomWidth="1px" borderColor="border.muted" bg="bg.panel" position="sticky" top={0} zIndex={10}>
        <Container maxW="6xl" px={{ base: 4, md: 6 }}>
          <Flex minH="56px" align="center" gap={{ base: 3, md: 6 }} wrap="wrap" py={2}>
            <HStack gap={2}>
              <Box bg="brand.solid" color="brand.contrast" borderRadius="md" p={1.5} display="flex">
                <Briefcase size={16} />
              </Box>
              <Text fontWeight="semibold" fontSize="md">
                Job Hunter
              </Text>
            </HStack>
            <HStack as="nav" gap={1} flex="1">
              {NAV.map((item) => {
                const active = item.matches.includes(route.page);
                return (
                  <Link
                    key={item.label}
                    href={href(item.route)}
                    px={3}
                    py={1.5}
                    borderRadius="md"
                    fontWeight="medium"
                    color={active ? "brand.fg" : "fg.muted"}
                    bg={active ? "brand.subtle" : undefined}
                    _hover={{ textDecoration: "none", bg: active ? "brand.subtle" : "bg.muted" }}
                    aria-current={active ? "page" : undefined}
                  >
                    {item.label}
                  </Link>
                );
              })}
            </HStack>
            <HStack gap={2}>
              <Text fontSize="xs" color="fg.muted" display={{ base: "none", md: "block" }}>
                {session?.email}
              </Text>
              <IconButton
                aria-label="Toggle color mode"
                variant="ghost"
                size="sm"
                onClick={() => setTheme(resolvedTheme === "dark" ? "light" : "dark")}
              >
                {resolvedTheme === "dark" ? <Sun size={16} /> : <Moon size={16} />}
              </IconButton>
              <Button variant="ghost" size="sm" onClick={() => void api.logout()}>
                <LogOut size={14} /> Sign out
              </Button>
            </HStack>
          </Flex>
        </Container>
      </Box>
      <Container as="main" maxW="6xl" px={{ base: 4, md: 6 }} py={{ base: 5, md: 8 }}>
        {children}
      </Container>
    </Box>
  );
}
