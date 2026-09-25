import { Box, Button, Flex, HStack, Link, SimpleGrid, Text, VStack } from "@chakra-ui/react";
import { timeAgo } from "@job-hunter/shared";
import { useQuery } from "@tanstack/react-query";
import { Download, Monitor } from "lucide-react";

import { ApplicationRow } from "../components/ApplicationRow";
import { ErrorBox, Loading, Panel } from "../components/State";
import { href } from "../route";
import { useSession } from "../session";
import { byUpdatedDesc, countByGroup, inGroup, type StatusGroupKey } from "../statusGroups";

const TILES: { key: StatusGroupKey; label: string; tone: string }[] = [
  { key: "review", label: "Awaiting approval", tone: "purple" },
  { key: "attention", label: "Needs attention", tone: "orange" },
  { key: "applied", label: "Applied", tone: "green" },
  { key: "interviews", label: "Interviews & offers", tone: "green" },
];

function formatSize(bytes: number): string {
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function Tile({ label, value, tone, link }: { label: string; value: number | string; tone?: string; link: string }) {
  const highlight = tone && typeof value === "number" && value > 0;
  return (
    <Link href={link} display="block" color="inherit" _hover={{ textDecoration: "none" }}>
      <Box borderWidth="1px" borderColor="border.muted" borderRadius="lg" p={4} bg="bg.panel" _hover={{ bg: "bg.muted" }}>
        <Text fontSize="xs" color="fg.muted">
          {label}
        </Text>
        <Text fontSize="2xl" fontWeight="semibold" fontVariantNumeric="tabular-nums" color={highlight ? `${tone}.fg` : undefined}>
          {value}
        </Text>
      </Box>
    </Link>
  );
}

export function OverviewPage() {
  const { api } = useSession();
  const applications = useQuery({ queryKey: ["applications"], queryFn: () => api.applications(), refetchInterval: 30_000 });
  const devices = useQuery({ queryKey: ["devices"], queryFn: () => api.devices(), refetchInterval: 30_000 });
  const jobs = useQuery({ queryKey: ["jobs"], queryFn: () => api.jobs(), refetchInterval: 60_000 });
  const android = useQuery({ queryKey: ["download", "android"], queryFn: () => api.androidDownload(), staleTime: 5 * 60_000 });

  if (applications.isPending) return <Loading />;
  if (applications.isError) return <ErrorBox error={applications.error} onRetry={() => void applications.refetch()} />;

  const items = applications.data;
  const counts = countByGroup(items);
  const needsYou = items.filter((i) => inGroup(i, "review") || inGroup(i, "attention")).sort(byUpdatedDesc).slice(0, 6);
  const recent = [...items].sort(byUpdatedDesc).slice(0, 6);

  const desktops = (devices.data ?? []).filter((d) => d.kind === "desktop");
  const online = desktops.some((d) => d.online);
  const lastSeen = desktops.map((d) => d.lastSeenAt).filter((x): x is string => !!x).sort().at(-1) ?? null;
  const desktopLabel = devices.isPending
    ? "Checking desktop…"
    : online
      ? "Desktop online"
      : lastSeen
        ? `Desktop offline · seen ${timeAgo(lastSeen)}`
        : "Desktop offline";

  return (
    <VStack align="stretch" gap={6}>
      <Flex justify="space-between" align={{ base: "flex-start", md: "center" }} gap={3} direction={{ base: "column", md: "row" }}>
        <Box>
          <Text fontSize="2xl" fontWeight="semibold">
            Overview
          </Text>
          <Text color="fg.muted">A read-only view of your job hunt. Approve, reject and apply from the desktop or mobile app.</Text>
        </Box>
        <HStack gap={2} px={3} py={1.5} borderRadius="full" borderWidth="1px" borderColor="border.muted" flexShrink={0}>
          <Monitor size={14} />
          <Box w={2} h={2} borderRadius="full" bg={online ? "green.solid" : "gray.solid"} />
          <Text fontSize="xs" fontWeight="medium">
            {desktopLabel}
          </Text>
        </HStack>
      </Flex>

      <SimpleGrid columns={{ base: 2, md: 5 }} gap={3}>
        {TILES.map((t) => (
          <Tile key={t.key} label={t.label} value={counts[t.key]} tone={t.tone} link={href({ page: "applications", group: t.key })} />
        ))}
        <Tile label="Jobs found" value={jobs.data?.length ?? "–"} link={href({ page: "jobs" })} />
      </SimpleGrid>

      <SimpleGrid columns={{ base: 1, lg: 2 }} gap={4}>
        <Panel title="Needs you">
          {needsYou.length === 0 ? (
            <Text fontSize="sm" color="fg.muted">
              Nothing is waiting on you.
            </Text>
          ) : (
            <VStack align="stretch" gap={0} mx={-3}>
              {needsYou.map((item) => (
                <ApplicationRow key={item.application.id} item={item} />
              ))}
            </VStack>
          )}
        </Panel>
        <Panel
          title="Recent activity"
          action={
            <Link href={href({ page: "applications" })} fontSize="xs" color="brand.fg">
              View all
            </Link>
          }
        >
          {recent.length === 0 ? (
            <Text fontSize="sm" color="fg.muted">
              No applications yet. Start a job hunt from the desktop app.
            </Text>
          ) : (
            <VStack align="stretch" gap={0} mx={-3}>
              {recent.map((item) => (
                <ApplicationRow key={item.application.id} item={item} />
              ))}
            </VStack>
          )}
        </Panel>
      </SimpleGrid>

      {android.data ? (
        <Panel title="Android app">
          <Flex gap={3} align={{ base: "flex-start", md: "center" }} justify="space-between" direction={{ base: "column", md: "row" }}>
            <Text fontSize="sm" color="fg.muted">
              Review and approve applications from your phone. {android.data.fileName} · {formatSize(android.data.sizeBytes)} · updated{" "}
              {timeAgo(android.data.updatedAt)}
            </Text>
            <Button asChild colorPalette="brand" size="sm" flexShrink={0}>
              <a href={android.data.url} download>
                <Download size={14} /> Download APK
              </a>
            </Button>
          </Flex>
        </Panel>
      ) : null}
    </VStack>
  );
}
