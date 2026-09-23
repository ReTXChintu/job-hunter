import { Box, Text } from "@chakra-ui/react";
import type { ReactNode } from "react";

export function KeyValue({ label, children }: { label: string; children: ReactNode }) {
  return (
    <Box>
      <Text fontSize="xs" color="fg.muted" textTransform="uppercase" letterSpacing="wide" fontWeight="semibold">
        {label}
      </Text>
      <Box fontSize="sm" mt={0.5}>
        {children}
      </Box>
    </Box>
  );
}
