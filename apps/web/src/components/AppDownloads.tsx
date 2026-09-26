import { Box, Button, Flex, Text, VStack } from "@chakra-ui/react";
import { timeAgo } from "@job-hunter/shared";
import { useQuery } from "@tanstack/react-query";
import { Download } from "lucide-react";

import type { DownloadInfo } from "../api";
import { useSession } from "../session";

function formatSize(bytes: number): string {
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

const APPS = [
  { key: "android", label: "Android", blurb: "Review and approve applications from your phone.", button: "Download APK" },
  { key: "windows", label: "Windows", blurb: "The desktop app that runs the job hunt.", button: "Download installer" },
] as const;

/** The latest Android and Windows builds on this server. Works signed out. */
export function useDownloads() {
  const { api } = useSession();
  return useQuery({ queryKey: ["downloads"], queryFn: () => api.downloads(), staleTime: 5 * 60_000 });
}

/** One row per available build; renders nothing until at least one exists. */
export function AppDownloads({ compact = false }: { compact?: boolean }) {
  const downloads = useDownloads();
  const available = APPS.map((app) => ({ ...app, info: downloads.data?.[app.key] ?? null })).filter(
    (app): app is (typeof APPS)[number] & { info: DownloadInfo } => app.info !== null,
  );
  if (available.length === 0) return null;

  return (
    <VStack align="stretch" gap={compact ? 3 : 4}>
      {available.map(({ key, label, blurb, button, info }) => (
        <Flex key={key} gap={3} align={{ base: "flex-start", md: "center" }} justify="space-between" direction={{ base: "column", sm: "row" }}>
          <Box minW={0}>
            <Text fontSize="sm" fontWeight="medium">
              {label}
              {info.version ? ` · v${info.version}` : ""}
            </Text>
            <Text fontSize="xs" color="fg.muted">
              {compact ? formatSize(info.sizeBytes) : `${blurb} ${formatSize(info.sizeBytes)} · updated ${timeAgo(info.updatedAt)}`}
            </Text>
          </Box>
          <Button asChild colorPalette="brand" size="sm" variant={key === "android" ? "solid" : "outline"} flexShrink={0}>
            <a href={info.url} download>
              <Download size={14} /> {button}
            </a>
          </Button>
        </Flex>
      ))}
    </VStack>
  );
}
