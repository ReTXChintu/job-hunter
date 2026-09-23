import { Box, Flex } from "@chakra-ui/react";
import { Outlet, useNavigate, useRouterState } from "@tanstack/react-router";
import { useEffect } from "react";
import { Sidebar } from "./Sidebar";
import { AgentStatusBar } from "./AgentStatusBar";
import { useSetupStatus } from "../lib/queries";
import { isTauri } from "../lib/tauri";
import { NotInTauri } from "./NotInTauri";

export function AppShell() {
  const setup = useSetupStatus();
  const navigate = useNavigate();
  const path = useRouterState({ select: (s) => s.location.pathname });

  // First-run: send the user to the setup wizard until it is completed.
  useEffect(() => {
    if (setup.data && !setup.data.setupCompleted && path !== "/setup") {
      void navigate({ to: "/setup" });
    }
  }, [setup.data, path, navigate]);

  if (!isTauri()) return <NotInTauri />;

  return (
    <Flex h="100vh" w="100vw" overflow="hidden" bg="bg">
      <Sidebar />
      <Flex direction="column" flex="1" minW={0}>
        <Box flex="1" overflowY="auto" bg="bg.canvas">
          <Box maxW="1400px" mx="auto" px={8} py={6}>
            <Outlet />
          </Box>
        </Box>
        <AgentStatusBar />
      </Flex>
    </Flex>
  );
}
