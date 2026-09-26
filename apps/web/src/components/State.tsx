import { Alert, Box, Button, Center, Spinner, Text } from "@chakra-ui/react";
import type { ReactNode } from "react";

export function Loading() {
  return (
    <Center py={16}>
      <Spinner color="brand.solid" />
    </Center>
  );
}

export function ErrorBox({ error, onRetry, title = "Couldn't load this" }: { error: unknown; onRetry?: () => void; title?: string }) {
  const message = error instanceof Error ? error.message : "Something went wrong.";
  return (
    <Alert.Root status="error" borderRadius="md" my={4}>
      <Alert.Indicator />
      <Alert.Content>
        <Alert.Title>{title}</Alert.Title>
        <Alert.Description>{message}</Alert.Description>
      </Alert.Content>
      {onRetry ? (
        <Button size="xs" variant="outline" onClick={onRetry}>
          Retry
        </Button>
      ) : null}
    </Alert.Root>
  );
}

export function Panel({ title, action, children }: { title: string; action?: ReactNode; children: ReactNode }) {
  return (
    <Box borderWidth="1px" borderColor="border.muted" borderRadius="lg" bg="bg.panel" p={{ base: 4, md: 5 }}>
      <Box display="flex" alignItems="center" justifyContent="space-between" mb={3} gap={3}>
        <Text fontWeight="semibold">{title}</Text>
        {action}
      </Box>
      {children}
    </Box>
  );
}
