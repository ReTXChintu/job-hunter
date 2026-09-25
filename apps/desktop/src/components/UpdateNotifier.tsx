import { Box, Button, CloseButton, HStack, Progress, Text } from "@chakra-ui/react";
import type { UpdateCheck, UpdateProgress } from "@job-hunter/agent-protocol";
import { Download, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";

import { useInstallUpdate, useOpenUrl, useUpdateCheck } from "../lib/queries";
import { listen } from "../lib/tauri";
import { ErrorBanner } from "./common";

const DISMISSED_KEY = "job-hunter.update-dismissed";
/** "Remind me later" hides the banner for this long, then it comes back. */
const REMIND_AFTER_MS = 3 * 24 * 60 * 60 * 1000;

interface Dismissal {
  version: string;
  until: number;
}

function formatMb(bytes: number): string {
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * "Install & restart" when this build can verify and install the update
 * itself; otherwise "Download" opens the installer from the server.
 */
export function UpdateActions({ check, size = "sm", compactError = false }: { check: UpdateCheck; size?: "xs" | "sm"; compactError?: boolean }) {
  const install = useInstallUpdate();
  const openUrl = useOpenUrl();
  const [progress, setProgress] = useState<UpdateProgress | null>(null);

  useEffect(() => {
    if (!install.isPending) return;
    const unlisten = listen("update:progress", setProgress);
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [install.isPending]);

  if (!check.available) return null;

  if (install.isPending) {
    const pct = progress?.total ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100)) : null;
    return (
      <Box minW="220px">
        <Text fontSize="xs" color="fg.muted" mb={1}>
          {progress?.finished ? "Installing… the app will restart." : pct !== null ? `Downloading ${pct}%` : "Downloading…"}
        </Text>
        <Progress.Root value={progress?.finished ? null : pct} size="xs" colorPalette="brand">
          <Progress.Track>
            <Progress.Range />
          </Progress.Track>
        </Progress.Root>
      </Box>
    );
  }

  return (
    <Box>
      <HStack gap={2}>
        {check.canInstallInApp ? (
          <Button size={size} colorPalette="brand" onClick={() => install.mutate()}>
            <RefreshCw size={14} /> Install &amp; restart
          </Button>
        ) : null}
        {check.downloadUrl ? (
          <Button size={size} variant={check.canInstallInApp ? "ghost" : "solid"} colorPalette="brand" onClick={() => openUrl.mutate(check.downloadUrl!)}>
            <Download size={14} /> Download{check.sizeBytes ? ` (${formatMb(check.sizeBytes)})` : ""}
          </Button>
        ) : null}
      </HStack>
      {install.error ? (
        compactError ? (
          <Text mt={1} fontSize="xs" color="red.fg" maxW="md" lineClamp={2} title={install.error.message}>
            Update failed: {install.error.message}
          </Text>
        ) : (
          <Box mt={2}>
            <ErrorBanner error={install.error} title="Update failed" />
          </Box>
        )
      ) : null}
    </Box>
  );
}

function readDismissal(): Dismissal | null {
  try {
    const parsed = JSON.parse(localStorage.getItem(DISMISSED_KEY) ?? "null") as Partial<Dismissal> | null;
    return parsed && typeof parsed.version === "string" && typeof parsed.until === "number" ? (parsed as Dismissal) : null;
  } catch {
    return null;
  }
}

/** A dismissible strip at the top of every page when a newer build is on the server. */
export function UpdateBanner() {
  const check = useUpdateCheck();
  const [dismissal, setDismissal] = useState(readDismissal);
  const data = check.data;
  const snoozed = dismissal !== null && dismissal.version === data?.latestVersion && Date.now() < dismissal.until;
  if (!data?.available || !data.latestVersion || snoozed) return null;

  const dismiss = () => {
    const next = { version: data.latestVersion!, until: Date.now() + REMIND_AFTER_MS };
    try {
      localStorage.setItem(DISMISSED_KEY, JSON.stringify(next));
    } catch {
      // Storage unavailable: dismissal lasts for this session only.
    }
    setDismissal(next);
  };

  return (
    <HStack px={8} py={2.5} gap={4} bg="brand.subtle" borderBottomWidth="1px" borderColor="brand.muted" wrap="wrap">
      <Text flex="1" fontSize="sm" minW="240px">
        <Text as="span" fontWeight="semibold">
          Job Hunter {data.latestVersion} is available.
        </Text>{" "}
        <Text as="span" color="fg.muted">
          You have {data.currentVersion}.
        </Text>
      </Text>
      <UpdateActions check={data} compactError />
      <CloseButton size="xs" aria-label="Remind me later" onClick={dismiss} />
    </HStack>
  );
}
