import { Badge, Box, Button, Flex, Heading, HStack, SimpleGrid, Spinner, Switch, Tag, Text, VStack } from "@chakra-ui/react";
import { formatDateTime, isAgentRunning } from "@job-hunter/shared";
import type { ApplicationAnswer, PlatformProfileView, PlatformSyncStatus } from "@job-hunter/types";
import { ExternalLink, RefreshCw, Send } from "lucide-react";
import { useState } from "react";
import { ActivityFeed } from "../components/ActivityFeed";
import { BulletList, ErrorBanner, InfoBanner, PageHeader, Panel } from "../components/common";
import { ProjectPicker } from "../components/ProjectPicker";
import { QuestionField } from "../components/QuestionField";
import { useActivity } from "../lib/activity";
import { useAgentStatus, useAnswerPlatformQuestions, useOpenUrl, usePlatformProfiles, useProjects, usePublishingPlan, useSetPlatformAutoSync, useSyncPlatformProfile } from "../lib/queries";

const STATUS: Record<PlatformSyncStatus, { label: string; palette: string }> = {
  NEVER_SYNCED: { label: "Not updated yet", palette: "gray" },
  SYNCING: { label: "Updating…", palette: "purple" },
  SYNCED: { label: "Up to date", palette: "green" },
  NEEDS_INPUT: { label: "Needs your answers", palette: "orange" },
  MANUAL_ACTION_REQUIRED: { label: "Needs you in Chrome", palette: "orange" },
  FAILED: { label: "Failed", palette: "red" },
};

export function JobSitesPage() {
  const platforms = usePlatformProfiles();
  const plan = usePublishingPlan();
  const projects = useProjects();
  const agent = useAgentStatus();
  const sync = useSyncPlatformProfile();
  const [picker, setPicker] = useState<{ open: boolean; platform: string | null }>({ open: false, platform: null });

  const busy = agent.data ? isAgentRunning(agent.data.state) : false;
  const confirmed = !!plan.data?.confirmedAt;
  const featured = (plan.data?.featuredProjectIds ?? []).map((id) => projects.data?.find((p) => p.id === id)?.name).filter((n): n is string => !!n);
  const update = (platform: string) => (confirmed ? sync.mutate(platform) : setPicker({ open: true, platform }));

  return (
    <Box>
      <PageHeader
        title="Job sites"
        subtitle="Fill in your own LinkedIn, Naukri, Indeed and Wellfound profiles from this app, so applications don't stop to ask for details and every site says the same thing."
      />
      <ErrorBanner error={sync.error ?? platforms.error ?? plan.error} />

      <Panel
        title="Projects on your profiles"
        action={
          <Button size="xs" variant="ghost" onClick={() => setPicker({ open: true, platform: null })}>
            {confirmed ? "Change" : "Choose with Claude"}
          </Button>
        }
      >
        {confirmed ? (
          featured.length ? (
            <Flex wrap="wrap" gap={1.5}>
              {featured.map((name) => (
                <Tag.Root key={name} size="md" variant="subtle" colorPalette="brand">
                  <Tag.Label>{name}</Tag.Label>
                </Tag.Root>
              ))}
            </Flex>
          ) : (
            <Text fontSize="sm" color="fg.muted">
              No projects: your profiles show experience and skills only.
            </Text>
          )
        ) : (
          <Text fontSize="sm" color="fg.muted">
            Not chosen yet. Claude will suggest your strongest few projects the first time you update a profile, and you decide before anything is published.
          </Text>
        )}
      </Panel>

      {platforms.isLoading ? (
        <Spinner mt={4} />
      ) : (
        <SimpleGrid columns={{ base: 1, xl: 2 }} gap={4} mt={4}>
          {platforms.data?.map((view) => (
            <PlatformCard key={view.profile.id} view={view} busy={busy} onUpdate={() => update(view.profile.platform)} updating={sync.isPending && sync.variables === view.profile.platform} />
          ))}
        </SimpleGrid>
      )}

      <ProjectPicker
        open={picker.open}
        platform={picker.platform}
        onClose={() => setPicker({ open: false, platform: null })}
        onConfirmed={(platform) => {
          if (platform) sync.mutate(platform);
        }}
      />
    </Box>
  );
}

function PlatformCard({ view, busy, updating, onUpdate }: { view: PlatformProfileView; busy: boolean; updating: boolean; onUpdate: () => void }) {
  const { profile, outOfDate } = view;
  const agent = useAgentStatus();
  const activity = useActivity();
  const openUrl = useOpenUrl();
  const autoSync = useSetPlatformAutoSync();
  const answer = useAnswerPlatformQuestions();
  const [answers, setAnswers] = useState<Record<string, string>>({});

  const status = outOfDate && profile.status === "SYNCED" ? { label: "Behind your profile", palette: "orange" } : STATUS[profile.status];
  const running = profile.status === "SYNCING" && agent.data?.runKind === "PROFILE_SYNC" && agent.data.runId === profile.runId && busy;
  const runEvents = running ? activity.filter((e) => e.runId === profile.runId).slice(-20) : [];
  const neverSynced = !profile.lastSyncedAt;

  const submitAnswers = () => {
    const list: ApplicationAnswer[] = profile.pendingQuestions.map((q) => ({ question: q.question, answer: (answers[q.id] ?? "").trim(), source: "USER" }));
    answer.mutate({ platform: profile.platform, answers: list.filter((a) => a.answer) }, { onSuccess: () => setAnswers({}) });
  };
  const missingRequired = profile.pendingQuestions.some((q) => q.required && !(answers[q.id] ?? "").trim());

  return (
    <Panel>
      <VStack align="stretch" gap={3}>
        <HStack justify="space-between" align="flex-start">
          <Box>
            <Heading size="md">{profile.platform}</Heading>
            <Text fontSize="xs" color="fg.muted">
              {profile.lastSyncedAt ? `Last updated ${formatDateTime(profile.lastSyncedAt)}` : "Never updated by Job Hunter"}
            </Text>
          </Box>
          <Badge colorPalette={status.palette} variant="subtle" textTransform="none">
            {status.label}
          </Badge>
        </HStack>

        {profile.message && profile.status !== "SYNCED" ? (
          <Text fontSize="sm" color={profile.status === "FAILED" ? "red.fg" : "fg.muted"}>
            {profile.message}
          </Text>
        ) : null}
        {profile.status === "MANUAL_ACTION_REQUIRED" ? (
          <InfoBanner status="warning" title="Finish this in Chrome">
            Sign in to {profile.platform} in Chrome (or clear the check it's showing), then update again.
          </InfoBanner>
        ) : null}

        {running ? <ActivityFeed events={runEvents} maxHeight="180px" compact /> : null}

        {profile.status === "NEEDS_INPUT" && profile.pendingQuestions.length ? (
          <Box p={3} borderRadius="md" bg="orange.subtle">
            <Text fontSize="sm" fontWeight="semibold" mb={2}>
              {profile.platform} asks for details your profile doesn't have
            </Text>
            <VStack align="stretch" gap={3}>
              {profile.pendingQuestions.map((q) => (
                <QuestionField key={q.id} q={q} value={answers[q.id] ?? ""} onChange={(v) => setAnswers((a) => ({ ...a, [q.id]: v }))} />
              ))}
              <HStack justify="space-between">
                <Text fontSize="xs" color="fg.muted">
                  Saved to your answers, so applications can use them too.
                </Text>
                <Button size="sm" colorPalette="brand" onClick={submitAnswers} loading={answer.isPending} disabled={busy || missingRequired}>
                  <Send size={14} /> Save &amp; continue
                </Button>
              </HStack>
              <ErrorBanner error={answer.error} />
            </VStack>
          </Box>
        ) : null}

        {profile.changes.length && !running ? (
          <Box>
            <Text fontSize="xs" color="fg.muted" fontWeight="semibold" textTransform="uppercase" letterSpacing="wide" mb={1}>
              Last changes
            </Text>
            <BulletList items={profile.changes.slice(0, 6)} />
            {profile.changes.length > 6 ? (
              <Text fontSize="xs" color="fg.muted">
                +{profile.changes.length - 6} more
              </Text>
            ) : null}
            {profile.skipped.length ? (
              <Text fontSize="xs" color="fg.subtle" mt={1}>
                Skipped: {profile.skipped.join("; ")}
              </Text>
            ) : null}
          </Box>
        ) : null}

        <HStack justify="space-between" wrap="wrap" gap={2}>
          <Switch.Root
            size="sm"
            checked={profile.autoSync}
            disabled={neverSynced || autoSync.isPending}
            onCheckedChange={(e) => autoSync.mutate({ platform: profile.platform, enabled: !!e.checked })}
            title={neverSynced ? "Available after the first update" : undefined}
          >
            <Switch.HiddenInput />
            <Switch.Control />
            <Switch.Label fontSize="sm" color={neverSynced ? "fg.subtle" : "fg"}>
              Keep in sync automatically
            </Switch.Label>
          </Switch.Root>
          <HStack gap={2}>
            {profile.profileUrl ? (
              <Button size="sm" variant="outline" onClick={() => openUrl.mutate(profile.profileUrl)}>
                <ExternalLink size={14} /> Open
              </Button>
            ) : null}
            <Button size="sm" colorPalette="brand" onClick={onUpdate} loading={updating || running} disabled={busy}>
              <RefreshCw size={14} /> {neverSynced ? "Update profile" : "Update again"}
            </Button>
          </HStack>
        </HStack>
      </VStack>
    </Panel>
  );
}
