import { Box, Heading, Text, VStack } from "@chakra-ui/react";
import type { ReactNode } from "react";

export function EmptyState({ title, description, action, icon }: { title: string; description?: string; action?: ReactNode; icon?: ReactNode }) {
  return (
    <VStack py={12} px={6} gap={3} textAlign="center" borderWidth="1px" borderStyle="dashed" borderColor="border" borderRadius="md" color="fg.muted">
      {icon ? <Box color="fg.subtle">{icon}</Box> : null}
      <Heading size="md" color="fg">
        {title}
      </Heading>
      {description ? (
        <Text maxW="md" fontSize="sm">
          {description}
        </Text>
      ) : null}
      {action ? <Box pt={2}>{action}</Box> : null}
    </VStack>
  );
}
