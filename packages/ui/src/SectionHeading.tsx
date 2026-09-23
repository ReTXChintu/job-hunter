import { Flex, Heading, Text } from "@chakra-ui/react";
import type { ReactNode } from "react";

export function SectionHeading({ title, subtitle, action }: { title: string; subtitle?: string; action?: ReactNode }) {
  return (
    <Flex align="flex-end" justify="space-between" gap={4} mb={3}>
      <div>
        <Heading size="sm" letterSpacing="wide" textTransform="uppercase" color="fg.muted" fontWeight="semibold">
          {title}
        </Heading>
        {subtitle ? (
          <Text fontSize="sm" color="fg.muted" mt={1}>
            {subtitle}
          </Text>
        ) : null}
      </div>
      {action}
    </Flex>
  );
}
