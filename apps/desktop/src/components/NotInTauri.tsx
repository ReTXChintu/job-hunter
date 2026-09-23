import { Box, Code, Heading, Text, VStack } from "@chakra-ui/react";

export function NotInTauri() {
  return (
    <VStack h="100vh" justify="center" gap={3} px={6} textAlign="center">
      <Heading size="lg">Job Hunter runs as a desktop app</Heading>
      <Text color="fg.muted" maxW="lg">
        This page is being served by the Vite dev server outside of Tauri, so the Rust backend is not available. Start the desktop application instead:
      </Text>
      <Box>
        <Code>pnpm tauri dev</Code>
      </Box>
    </VStack>
  );
}
