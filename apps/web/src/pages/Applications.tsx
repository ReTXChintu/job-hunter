import { Box, Button, Flex, Input, Text, VStack, Wrap } from "@chakra-ui/react";
import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";

import { ApplicationRow } from "../components/ApplicationRow";
import { ErrorBox, Loading } from "../components/State";
import { href } from "../route";
import { useSession } from "../session";
import { byUpdatedDesc, countByGroup, inGroup, STATUS_GROUPS, type StatusGroupKey } from "../statusGroups";

function isGroupKey(value: string | undefined): value is StatusGroupKey {
  return STATUS_GROUPS.some((g) => g.key === value);
}

export function ApplicationsPage({ group }: { group?: string }) {
  const { api } = useSession();
  const applications = useQuery({ queryKey: ["applications"], queryFn: () => api.applications(), refetchInterval: 30_000 });
  const [query, setQuery] = useState("");
  const active: StatusGroupKey = isGroupKey(group) ? group : "all";

  const counts = useMemo(() => countByGroup(applications.data ?? []), [applications.data]);
  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (applications.data ?? [])
      .filter((item) => inGroup(item, active))
      .filter((item) => !q || `${item.job.title} ${item.job.company} ${item.job.location}`.toLowerCase().includes(q))
      .sort(byUpdatedDesc);
  }, [applications.data, active, query]);

  return (
    <VStack align="stretch" gap={5}>
      <Box>
        <Text fontSize="2xl" fontWeight="semibold">
          Applications
        </Text>
        <Text color="fg.muted">Everything the agent has prepared or submitted, newest activity first.</Text>
      </Box>

      <Flex gap={3} direction={{ base: "column", md: "row" }} align={{ base: "stretch", md: "center" }} justify="space-between">
        <Wrap gap={2}>
          {STATUS_GROUPS.map((g) => (
            <Button
              key={g.key}
              asChild
              size="xs"
              variant={g.key === active ? "solid" : "outline"}
              colorPalette={g.key === active ? "brand" : "gray"}
            >
              <a href={href({ page: "applications", group: g.key === "all" ? undefined : g.key })}>
                {g.label} · {counts[g.key]}
              </a>
            </Button>
          ))}
        </Wrap>
        <Input size="sm" maxW={{ md: "260px" }} placeholder="Search company, role, location" value={query} onChange={(e) => setQuery(e.target.value)} />
      </Flex>

      {applications.isPending ? (
        <Loading />
      ) : applications.isError ? (
        <ErrorBox error={applications.error} onRetry={() => void applications.refetch()} />
      ) : visible.length === 0 ? (
        <Box borderWidth="1px" borderColor="border.muted" borderRadius="lg" p={8} textAlign="center">
          <Text color="fg.muted">{applications.data.length === 0 ? "No applications yet. Start a job hunt from the desktop app." : "Nothing matches this filter."}</Text>
        </Box>
      ) : (
        <Box borderWidth="1px" borderColor="border.muted" borderRadius="lg" bg="bg.panel" p={2}>
          <VStack align="stretch" gap={0}>
            {visible.map((item) => (
              <ApplicationRow key={item.application.id} item={item} />
            ))}
          </VStack>
        </Box>
      )}
    </VStack>
  );
}
