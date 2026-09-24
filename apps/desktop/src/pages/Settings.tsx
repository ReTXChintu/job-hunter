import { Box, Button, Field, Heading, HStack, Input, SimpleGrid, Spinner, Switch, Tabs, Text, VStack } from "@chakra-ui/react";
import { formatDateTime } from "@job-hunter/shared";
import type { AppSettings } from "@job-hunter/types";
import { Database, Save } from "lucide-react";
import { useEffect, useState } from "react";
import { ErrorBanner, InfoBanner, PageHeader, Panel } from "../components/common";
import { MobileAppSettings } from "../components/MobileAppSettings";
import { LogViewer } from "./Agent";
import { useClearMongo, useConfigureMongo, useSaveSettings, useSettings, useSetupStatus, useSyncStatus, useTestMongo } from "../lib/queries";

export function SettingsPage() {
  const settingsQ = useSettings();
  const save = useSaveSettings();
  const [draft, setDraft] = useState<AppSettings | null>(null);
  const [dirty, setDirty] = useState(false);
  useEffect(() => {
    if (settingsQ.data && !dirty) setDraft(structuredClone(settingsQ.data));
  }, [settingsQ.data, dirty]);
  if (!draft) return settingsQ.error ? <ErrorBanner error={settingsQ.error} /> : <Spinner />;
  const update = (fn: (s: AppSettings) => void) => {
    setDraft((d) => {
      if (!d) return d;
      const n = structuredClone(d);
      fn(n);
      return n;
    });
    setDirty(true);
  };
  const num = (v: string, fallback: number) => (v === "" || Number.isNaN(Number(v)) ? fallback : Number(v));

  return (
    <Box>
      <PageHeader
        title="Settings"
        actions={
          <Button colorPalette="brand" onClick={() => save.mutate(draft, { onSuccess: () => setDirty(false) })} loading={save.isPending} disabled={!dirty}>
            <Save size={16} /> Save settings
          </Button>
        }
      />
      <ErrorBanner error={save.error} />
      <Tabs.Root defaultValue="mongo" variant="line" size="sm" lazyMount unmountOnExit>
        <Tabs.List mb={4}>
          <Tabs.Trigger value="mongo">MongoDB Atlas</Tabs.Trigger>
          <Tabs.Trigger value="sources">Job sources</Tabs.Trigger>
          <Tabs.Trigger value="agent">Agent</Tabs.Trigger>
          <Tabs.Trigger value="resume">Resume</Tabs.Trigger>
          <Tabs.Trigger value="integrations">Claude &amp; Chrome</Tabs.Trigger>
          <Tabs.Trigger value="mobile">Mobile app</Tabs.Trigger>
          <Tabs.Trigger value="logs">Logs</Tabs.Trigger>
        </Tabs.List>

        <Tabs.Content value="mongo">
          <MongoSettings database={draft.mongodbDatabase} onDatabaseChange={(v) => update((s) => { s.mongodbDatabase = v; })} />
        </Tabs.Content>

        <Tabs.Content value="sources">
          <Panel title="Job sources">
            <Text fontSize="sm" color="fg.muted" mb={4}>
              Each enabled source is searched through Claude in Chrome using your own logged-in browser session. Start with one or two sources; each search is a separate Claude run.
            </Text>
            <VStack align="stretch" gap={3}>
              {draft.jobSources.map((src, i) => (
                <Switch.Root key={src.platform} checked={src.enabled} onCheckedChange={(e) => update((s) => { s.jobSources[i]!.enabled = !!e.checked; })}>
                  <Switch.HiddenInput />
                  <Switch.Control />
                  <Switch.Label>{src.platform}</Switch.Label>
                </Switch.Root>
              ))}
            </VStack>
            <SimpleGrid columns={{ base: 1, md: 3 }} gap={4} mt={6}>
              <Field.Root>
                <Field.Label>Max jobs per source</Field.Label>
                <Input type="number" value={draft.maxJobsPerSource} onChange={(e) => update((s) => { s.maxJobsPerSource = num(e.target.value, 10); })} />
              </Field.Root>
              <Field.Root>
                <Field.Label>Only jobs posted in the last (days)</Field.Label>
                <Input type="number" value={draft.recencyDays} onChange={(e) => update((s) => { s.recencyDays = num(e.target.value, 7); })} />
              </Field.Root>
            </SimpleGrid>
          </Panel>
        </Tabs.Content>

        <Tabs.Content value="agent">
          <Panel title="Agent behaviour">
            <SimpleGrid columns={{ base: 1, md: 3 }} gap={4}>
              <Field.Root>
                <Field.Label>Max applications prepared per run</Field.Label>
                <Input type="number" value={draft.maxApplicationsPerRun} onChange={(e) => update((s) => { s.maxApplicationsPerRun = num(e.target.value, 10); })} />
              </Field.Root>
              <Field.Root>
                <Field.Label>Minimum match score to prepare (0-100)</Field.Label>
                <Input type="number" value={draft.minimumMatchScore} onChange={(e) => update((s) => { s.minimumMatchScore = Math.min(100, Math.max(0, num(e.target.value, 60))); })} />
              </Field.Root>
              <Field.Root>
                <Field.Label>Approval mode</Field.Label>
                <Input value="Review required (every submission needs your approval)" readOnly />
                <Field.HelperText>Automatic submission is intentionally not available.</Field.HelperText>
              </Field.Root>
            </SimpleGrid>
            <Box mt={6}>
              <Switch.Root checked={draft.mockMode} onCheckedChange={(e) => update((s) => { s.mockMode = !!e.checked; })} colorPalette="orange">
                <Switch.HiddenInput />
                <Switch.Control />
                <Switch.Label>Mock mode (development only: simulate Claude and Chrome with fixture data, never submit anything)</Switch.Label>
              </Switch.Root>
            </Box>
          </Panel>
        </Tabs.Content>

        <Tabs.Content value="resume">
          <Panel title="Resume generation">
            <VStack align="stretch" gap={4}>
              <Switch.Root checked={draft.resume.generateCoverLetter} onCheckedChange={(e) => update((s) => { s.resume.generateCoverLetter = !!e.checked; })}>
                <Switch.HiddenInput />
                <Switch.Control />
                <Switch.Label>Generate a cover letter with each resume</Switch.Label>
              </Switch.Root>
              <Switch.Root checked={draft.resume.generatePdf} onCheckedChange={(e) => update((s) => { s.resume.generatePdf = !!e.checked; })}>
                <Switch.HiddenInput />
                <Switch.Control />
                <Switch.Label>Produce PDF</Switch.Label>
              </Switch.Root>
              <Switch.Root checked={draft.resume.generateDocx} onCheckedChange={(e) => update((s) => { s.resume.generateDocx = !!e.checked; })}>
                <Switch.HiddenInput />
                <Switch.Control />
                <Switch.Label>Produce DOCX</Switch.Label>
              </Switch.Root>
              <SimpleGrid columns={{ base: 1, md: 2 }} gap={4}>
                <Field.Root>
                  <Field.Label>ATS validation iterations (max 3)</Field.Label>
                  <Input type="number" value={draft.resume.atsMaxIterations} onChange={(e) => update((s) => { s.resume.atsMaxIterations = Math.min(3, Math.max(1, num(e.target.value, 3))); })} />
                </Field.Root>
                <Field.Root>
                  <Field.Label>Target keyword coverage (%)</Field.Label>
                  <Input type="number" value={draft.resume.minimumKeywordCoverage} onChange={(e) => update((s) => { s.resume.minimumKeywordCoverage = Math.min(100, Math.max(0, num(e.target.value, 70))); })} />
                </Field.Root>
              </SimpleGrid>
            </VStack>
          </Panel>
        </Tabs.Content>

        <Tabs.Content value="integrations">
          <IntegrationSettings draft={draft} update={update} />
        </Tabs.Content>

        <Tabs.Content value="mobile">
          <MobileAppSettings />
        </Tabs.Content>

        <Tabs.Content value="logs">
          <LogViewer />
        </Tabs.Content>
      </Tabs.Root>
    </Box>
  );
}

function MongoSettings({ database, onDatabaseChange }: { database: string; onDatabaseChange: (v: string) => void }) {
  const sync = useSyncStatus();
  const configure = useConfigureMongo();
  const test = useTestMongo();
  const clear = useClearMongo();
  const [uri, setUri] = useState("");
  const [db, setDb] = useState(database);
  useEffect(() => setDb(database), [database]);
  const s = sync.data;
  return (
    <VStack align="stretch" gap={4}>
      <Panel title="Connection">
        <Text fontSize="sm" color="fg.muted" mb={4}>
          The connection string is stored in your operating system&apos;s credential manager, never in a file. Data is always kept locally and synced to Atlas when it is reachable.
        </Text>
        <ErrorBanner error={configure.error ?? test.error ?? clear.error} />
        {test.data ? <InfoBanner status="success">Connected to {test.data}</InfoBanner> : null}
        {configure.isSuccess ? <InfoBanner status="success">Saved. Existing local data has been queued for upload.</InfoBanner> : null}
        <Field.Root mb={3}>
          <Field.Label>Connection string (mongodb+srv://…)</Field.Label>
          <Input type="password" value={uri} placeholder={s?.configured ? "•••••••• (configured)" : "mongodb+srv://user:password@cluster.mongodb.net/"} onChange={(e) => setUri(e.target.value)} />
        </Field.Root>
        <Field.Root mb={4} maxW="320px">
          <Field.Label>Database name</Field.Label>
          <Input value={db} onChange={(e) => { setDb(e.target.value); onDatabaseChange(e.target.value); }} />
        </Field.Root>
        <HStack>
          <Button variant="outline" onClick={() => test.mutate({ uri, database: db })} loading={test.isPending} disabled={!uri}>
            Test connection
          </Button>
          <Button colorPalette="brand" onClick={() => configure.mutate({ uri, database: db }, { onSuccess: () => setUri("") })} loading={configure.isPending} disabled={!uri}>
            <Database size={14} /> Save &amp; connect
          </Button>
          {s?.configured ? (
            <Button variant="ghost" colorPalette="red" onClick={() => clear.mutate()} loading={clear.isPending}>
              Remove connection
            </Button>
          ) : null}
        </HStack>
      </Panel>
      <Panel title="Sync status">
        {s ? (
          <SimpleGrid columns={{ base: 2, md: 4 }} gap={3} fontSize="sm">
            <Box><Text color="fg.muted">Configured</Text><Text fontWeight="semibold">{s.configured ? "Yes" : "No"}</Text></Box>
            <Box><Text color="fg.muted">Connected</Text><Text fontWeight="semibold" color={s.connected ? "green.fg" : s.configured ? "orange.fg" : undefined}>{s.connected ? "Yes" : s.configured ? "Offline" : "—"}</Text></Box>
            <Box><Text color="fg.muted">Pending changes</Text><Text fontWeight="semibold">{s.pending}</Text></Box>
            <Box><Text color="fg.muted">Last sync</Text><Text fontWeight="semibold">{s.lastSyncAt ? formatDateTime(s.lastSyncAt) : "Never"}</Text></Box>
            {s.target ? <Box gridColumn="span 2"><Text color="fg.muted">Target</Text><Text fontFamily="mono" fontSize="xs">{s.target}</Text></Box> : null}
            {s.lastError ? <Box gridColumn="span 2"><Text color="fg.muted">Last error</Text><Text color="orange.fg">{s.lastError}</Text></Box> : null}
          </SimpleGrid>
        ) : <Spinner size="sm" />}
      </Panel>
    </VStack>
  );
}

function IntegrationSettings({ draft, update }: { draft: AppSettings; update: (fn: (s: AppSettings) => void) => void }) {
  const setup = useSetupStatus();
  const c = setup.data?.claude;
  const ch = setup.data?.chrome;
  return (
    <VStack align="stretch" gap={4}>
      <Panel title="Claude Code">
        {c ? (
          <SimpleGrid columns={{ base: 2, md: 4 }} gap={3} fontSize="sm" mb={4}>
            <Box><Text color="fg.muted">Installed</Text><Text fontWeight="semibold" color={c.installed ? "green.fg" : "red.fg"}>{c.installed ? `Yes (${c.version})` : "No"}</Text></Box>
            <Box><Text color="fg.muted">Signed in</Text><Text fontWeight="semibold" color={c.authenticated ? "green.fg" : "red.fg"}>{c.authenticated ? `Yes${c.account ? ` · ${c.account}` : ""}` : "No"}</Text></Box>
            <Box gridColumn="span 2"><Text color="fg.muted">Executable</Text><Text fontFamily="mono" fontSize="xs">{c.path ?? "not found"}</Text></Box>
            {c.error ? <Box gridColumn="span 4"><Text color="orange.fg">{c.error}</Text></Box> : null}
          </SimpleGrid>
        ) : <Spinner size="sm" />}
        <SimpleGrid columns={{ base: 1, md: 2 }} gap={4}>
          <Field.Root>
            <Field.Label>Claude CLI path (optional override)</Field.Label>
            <Input value={draft.claude.cliPath ?? ""} placeholder="auto-detect" onChange={(e) => update((s) => { s.claude.cliPath = e.target.value || null; })} />
          </Field.Root>
          <Field.Root>
            <Field.Label>Model (optional, e.g. sonnet, opus)</Field.Label>
            <Input value={draft.claude.model ?? ""} placeholder="Claude Code default" onChange={(e) => update((s) => { s.claude.model = e.target.value || null; })} />
          </Field.Root>
          <Field.Root>
            <Field.Label>Max spend per Claude call (USD, 0 = unlimited)</Field.Label>
            <Input type="number" step="0.5" value={draft.claude.maxBudgetUsdPerCall} onChange={(e) => update((s) => { s.claude.maxBudgetUsdPerCall = Number(e.target.value) || 0; })} />
          </Field.Root>
          <Field.Root>
            <Field.Label>Max browser turns per task</Field.Label>
            <Input type="number" value={draft.claude.maxTurnsBrowser} onChange={(e) => update((s) => { s.claude.maxTurnsBrowser = Number(e.target.value) || 80; })} />
          </Field.Root>
          <Field.Root>
            <Field.Label>Timeout per Claude call (seconds)</Field.Label>
            <Input type="number" value={draft.claude.timeoutSeconds} onChange={(e) => update((s) => { s.claude.timeoutSeconds = Number(e.target.value) || 900; })} />
          </Field.Root>
        </SimpleGrid>
        <Text fontSize="xs" color="fg.muted" mt={3}>
          Job Hunter never uses an Anthropic API key. All AI work runs through your signed-in Claude Code installation.
        </Text>
      </Panel>
      <Panel title="Google Chrome &amp; Claude in Chrome">
        {ch ? (
          <SimpleGrid columns={{ base: 2, md: 4 }} gap={3} fontSize="sm" mb={4}>
            <Box><Text color="fg.muted">Chrome</Text><Text fontWeight="semibold" color={ch.installed ? "green.fg" : "red.fg"}>{ch.installed ? `Found${ch.version ? ` (${ch.version})` : ""}` : "Not found"}</Text></Box>
            <Box><Text color="fg.muted">Claude in Chrome extension</Text><Text fontWeight="semibold" color={ch.extensionInstalled ? "green.fg" : "orange.fg"}>{ch.extensionInstalled ? `Installed (${ch.extensionVersion})` : "Not detected"}</Text></Box>
            <Box gridColumn="span 2"><Text color="fg.muted">Executable</Text><Text fontFamily="mono" fontSize="xs">{ch.path ?? "—"}</Text></Box>
          </SimpleGrid>
        ) : <Spinner size="sm" />}
        <Field.Root maxW="560px">
          <Field.Label>Chrome path (optional override)</Field.Label>
          <Input value={draft.browser.chromePath ?? ""} placeholder="auto-detect" onChange={(e) => update((s) => { s.browser.chromePath = e.target.value || null; })} />
        </Field.Root>
        <Heading size="xs" mt={4} mb={1}>How it works</Heading>
        <Text fontSize="sm" color="fg.muted">
          Claude Code drives your real Chrome window through the Claude in Chrome extension, so job sites see your normal logged-in session. Job Hunter never reads cookies or passwords and never bypasses CAPTCHAs.
        </Text>
      </Panel>
    </VStack>
  );
}
