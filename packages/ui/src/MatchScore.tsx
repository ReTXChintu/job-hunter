import { Box, HStack, Text } from "@chakra-ui/react";

export function MatchScore({ score, relevant, compact = false }: { score: number | null | undefined; relevant?: boolean | null; compact?: boolean }) {
  if (score === null || score === undefined) {
    return (
      <Text fontSize="xs" color="fg.muted">
        Not analyzed
      </Text>
    );
  }
  const palette = relevant === false ? "gray" : score >= 85 ? "green" : score >= 60 ? "blue" : "orange";
  return (
    <HStack gap={2} minW={compact ? "auto" : "120px"}>
      <Box flex="1" h="6px" bg="bg.muted" borderRadius="full" overflow="hidden" minW={compact ? "48px" : "64px"}>
        <Box h="100%" w={`${Math.max(0, Math.min(100, score))}%`} bg={`${palette}.solid`} />
      </Box>
      <Text fontSize="sm" fontWeight="semibold" fontVariantNumeric="tabular-nums" color={relevant === false ? "fg.muted" : "fg"}>
        {score}%
      </Text>
    </HStack>
  );
}
