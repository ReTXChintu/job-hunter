import { Box, Flex, Link, Text } from "@chakra-ui/react";
import { timeAgo } from "@job-hunter/shared";
import type { ApplicationListItem } from "@job-hunter/types";
import { MatchScore, StatusBadge } from "@job-hunter/ui";

import { href } from "../route";

export function ApplicationRow({ item }: { item: ApplicationListItem }) {
  const { application, job, analysis } = item;
  return (
    <Link
      href={href({ page: "application", id: application.id })}
      display="block"
      px={3}
      py={3}
      borderRadius="md"
      color="inherit"
      _hover={{ textDecoration: "none", bg: "bg.muted" }}
    >
      <Flex gap={3} align={{ base: "flex-start", md: "center" }} direction={{ base: "column", md: "row" }}>
        <Box flex="1" minW={0}>
          <Text fontWeight="medium" truncate>
            {job.title}
          </Text>
          <Text fontSize="xs" color="fg.muted" truncate>
            {job.company}
            {job.location ? ` · ${job.location}` : ""}
          </Text>
        </Box>
        <Flex gap={4} align="center" wrap="wrap">
          <MatchScore score={analysis?.matchScore} relevant={analysis?.relevant} compact />
          <StatusBadge status={application.status} />
          <Text fontSize="xs" color="fg.muted" minW="70px" textAlign={{ md: "right" }}>
            {timeAgo(application.updatedAt)}
          </Text>
        </Flex>
      </Flex>
    </Link>
  );
}
