import { Button, HStack, NativeSelect, Text } from "@chakra-ui/react";
import type { PlatformProfileView, ProfileSyncMode } from "@job-hunter/types";
import { RefreshCw } from "lucide-react";
import { ErrorBanner, Panel } from "./common";
import { useQueueProfileUpdates, useSaveSettings, useSettings } from "../lib/queries";

const MODES: { value: ProfileSyncMode; label: string }[] = [
  { value: "NIGHTLY", label: "Every night" },
  { value: "ASK", label: "Ask me first" },
  { value: "IMMEDIATE", label: "Right away" },
  { value: "OFF", label: "Off" },
];

const hourLabel = (h: number) => `${((h + 11) % 12) + 1}:00 ${h < 12 ? "AM" : "PM"}`;

/**
 * When profiles that fell behind your desktop profile get updated. Updates
 * take Chrome over for a while, so by default they wait for the night
 * instead of getting in the way of applications.
 */
export function AutoUpdatePanel({ views, busy }: { views: PlatformProfileView[]; busy: boolean }) {
  const settings = useSettings();
  const save = useSaveSettings();
  const queue = useQueueProfileUpdates();
  const behind = views.filter((v) => v.outOfDate && v.profile.autoSync && v.profile.status === "SYNCED");
  const queued = views.filter((v) => v.profile.queuedAt).length;
  const current = settings.data?.profileSync ?? { mode: "NIGHTLY" as const, nightlyHour: 0 };

  const update = (patch: Partial<typeof current>) => {
    if (!settings.data) return;
    save.mutate({ ...settings.data, profileSync: { ...current, ...patch } });
  };

  const explanation: Record<ProfileSyncMode, string> = {
    NIGHTLY: `Profiles that fell behind are updated one by one from ${hourLabel(current.nightlyHour)} (local time), when the agent is free, so they never interrupt applications during the day.`,
    ASK: "You get a notification when profiles fall behind; nothing runs until you click Update all now.",
    IMMEDIATE: "Profiles are updated as soon as you change your desktop profile and the agent is free.",
    OFF: "Profiles are only updated when you click Update on a site or Update all now.",
  };

  return (
    <Panel title="Automatic updates" mb={4}>
      <ErrorBanner error={save.error ?? queue.error} />
      <HStack gap={3} wrap="wrap" mb={2}>
        <NativeSelect.Root size="sm" width="auto">
          <NativeSelect.Field value={current.mode} onChange={(e) => update({ mode: e.target.value as ProfileSyncMode })} aria-label="When to update profiles automatically">
            {MODES.map((m) => (
              <option key={m.value} value={m.value}>
                {m.label}
              </option>
            ))}
          </NativeSelect.Field>
          <NativeSelect.Indicator />
        </NativeSelect.Root>
        {current.mode === "NIGHTLY" ? (
          <NativeSelect.Root size="sm" width="auto">
            <NativeSelect.Field value={current.nightlyHour} onChange={(e) => update({ nightlyHour: Number(e.target.value) })} aria-label="Start time">
              {Array.from({ length: 24 }, (_, h) => (
                <option key={h} value={h}>
                  from {hourLabel(h)}
                </option>
              ))}
            </NativeSelect.Field>
            <NativeSelect.Indicator />
          </NativeSelect.Root>
        ) : null}
        <Button
          size="sm"
          colorPalette="brand"
          variant={behind.length ? "solid" : "outline"}
          onClick={() => queue.mutate()}
          loading={queue.isPending}
          disabled={behind.length === 0}
          title={busy ? "They'll start one by one as soon as the agent is free" : undefined}
        >
          <RefreshCw size={14} /> Update all now{behind.length ? ` (${behind.length})` : ""}
        </Button>
      </HStack>
      <Text fontSize="sm" color="fg.muted">
        {explanation[current.mode]}
        {queued ? ` ${queued} queued: they start as soon as the agent is free.` : behind.length ? ` ${behind.length} behind your profile now.` : " All profiles are up to date."}
      </Text>
    </Panel>
  );
}
