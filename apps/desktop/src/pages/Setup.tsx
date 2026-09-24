import { Box, Button, Field, Heading, HStack, Input, Spinner, Text, VStack } from "@chakra-ui/react";
import { useNavigate } from "@tanstack/react-router";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Check, ChevronRight, CircleAlert, RefreshCw } from "lucide-react";
import { useState, type ReactNode } from "react";
import { ErrorBanner, InfoBanner } from "../components/common";
import { useBackendRegister, useImportMasterResume, useOpenUrl, useParseMasterResume, useSaveSettings, useSettings, useSetupStatus, useStartJobHunt } from "../lib/queries";

function Step({ n, title, ok, warn, children }: { n: number; title: string; ok: boolean; warn?: boolean; children: ReactNode }) {
  return (
    <HStack align="flex-start" gap={4} py={4} borderBottomWidth="1px" borderColor="border.muted">
      <Box w={8} h={8} borderRadius="full" display="flex" alignItems="center" justifyContent="center" flexShrink={0} bg={ok ? "green.solid" : warn ? "orange.solid" : "bg.muted"} color={ok || warn ? "white" : "fg.muted"} fontWeight="bold" fontSize="sm">
        {ok ? <Check size={16} /> : warn ? <CircleAlert size={16} /> : n}
      </Box>
      <Box flex="1">
        <Heading size="sm" mb={1}>
          Step {n} · {title}
        </Heading>
        {children}
      </Box>
    </HStack>
  );
}

export function SetupPage() {
  const setup = useSetupStatus();
  const settings = useSettings();
  const saveSettings = useSaveSettings();
  const register = useBackendRegister();
  const importResume = useImportMasterResume();
  const parse = useParseMasterResume();
  const start = useStartJobHunt();
  const openUrl = useOpenUrl();
  const navigate = useNavigate();
  const [backendUrl, setBackendUrl] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [backendSkipped, setBackendSkipped] = useState(false);
  const s = setup.data;
  if (!s) return setup.error ? <ErrorBanner error={setup.error} onRetry={() => setup.refetch()} /> : <Spinner />;

  const mock = s.mockMode;
  const claudeOk = mock || (s.claude.installed && s.claude.authenticated);
  const chromeOk = mock || (s.chrome.installed && s.chrome.extensionInstalled);
  const backendOk = s.backend.configured;
  const resumeOk = s.profile.hasMasterResume;
  const profileOk = s.profile.ready;
  const allReady = claudeOk && chromeOk && profileOk;

  const finish = async (thenStart: boolean) => {
    if (settings.data) await saveSettings.mutateAsync({ ...settings.data, setupCompleted: true });
    if (thenStart) start.mutate({});
    await navigate({ to: "/" });
  };

  const pick = async () => {
    const selected = await openDialog({ multiple: false, directory: false, filters: [{ name: "Resume", extensions: ["pdf", "docx"] }] });
    if (typeof selected === "string") importResume.mutate(selected);
  };

  return (
    <Box maxW="820px" mx="auto" py={4}>
      <Heading size="2xl" mb={1}>
        Welcome to Job Hunter
      </Heading>
      <Text color="fg.muted" mb={6}>
        A personal, local-first job hunting command center. It uses your own Claude Code login and your own Chrome, and never submits an application without your approval.
      </Text>
      {mock ? (
        <InfoBanner status="warning" title="Mock mode is on">
          Claude and Chrome are simulated with fixture data. Turn it off in Settings → Agent for real job hunts.
        </InfoBanner>
      ) : null}
      <ErrorBanner error={register.error ?? importResume.error ?? parse.error ?? saveSettings.error ?? start.error} />

      <Step n={1} title="Check Claude CLI" ok={s.claude.installed || mock} warn={!s.claude.installed && !mock}>
        {s.claude.installed ? (
          <Text fontSize="sm" color="fg.muted">
            Claude CLI {s.claude.version} found at {s.claude.path}
          </Text>
        ) : (
          <VStack align="flex-start" gap={2}>
            <Text fontSize="sm" color="fg.muted">
              Claude Code was not found. Install it, then click re-check.
            </Text>
            <HStack>
              <Button size="xs" variant="outline" onClick={() => openUrl.mutate("https://code.claude.com/docs/en/overview")}>
                Install instructions
              </Button>
              <Button size="xs" variant="ghost" onClick={() => setup.refetch()}>
                <RefreshCw size={12} /> Re-check
              </Button>
            </HStack>
          </VStack>
        )}
      </Step>

      <Step n={2} title="Check Claude authentication" ok={s.claude.authenticated || mock} warn={s.claude.installed && !s.claude.authenticated && !mock}>
        {s.claude.authenticated ? (
          <Text fontSize="sm" color="fg.muted">
            Signed in{s.claude.account ? ` as ${s.claude.account}` : ""}{s.claude.subscription ? ` (${s.claude.subscription})` : ""}. Job Hunter reuses this login; no API key is needed.
          </Text>
        ) : (
          <VStack align="flex-start" gap={2}>
            <Text fontSize="sm" color="fg.muted">
              Open a terminal, run <b>claude</b> and complete the sign-in (or <b>claude auth login</b>), then re-check.
            </Text>
            <Button size="xs" variant="ghost" onClick={() => setup.refetch()}>
              <RefreshCw size={12} /> Re-check
            </Button>
          </VStack>
        )}
      </Step>

      <Step n={3} title="Check Chrome and Claude in Chrome" ok={chromeOk} warn={!chromeOk}>
        <Text fontSize="sm" color="fg.muted">
          {s.chrome.installed ? `Chrome ${s.chrome.version ?? ""} found. ` : "Chrome not found. "}
          {s.chrome.extensionInstalled ? `Claude in Chrome extension ${s.chrome.extensionVersion} is installed.` : "Claude in Chrome extension not detected in your Chrome profiles."}
        </Text>
        {!chromeOk ? (
          <HStack mt={2}>
            {!s.chrome.installed ? (
              <Button size="xs" variant="outline" onClick={() => openUrl.mutate("https://www.google.com/chrome/")}>
                Get Chrome
              </Button>
            ) : null}
            {!s.chrome.extensionInstalled ? (
              <Button size="xs" variant="outline" onClick={() => openUrl.mutate("https://chromewebstore.google.com/detail/claude/fcoeoabgfenejglbffodgkkbkcdhcgfn")}>
                Install Claude in Chrome
              </Button>
            ) : null}
            <Button size="xs" variant="ghost" onClick={() => setup.refetch()}>
              <RefreshCw size={12} /> Re-check
            </Button>
          </HStack>
        ) : null}
      </Step>

      <Step n={4} title="Sign in to your backend (optional)" ok={backendOk} warn={!backendOk && backendSkipped}>
        {backendOk ? (
          <Text fontSize="sm" color="fg.muted">
            Connected as {s.backend.target}. Everything is stored locally and synced to your backend.
          </Text>
        ) : (
          <VStack align="stretch" gap={2} maxW="560px">
            <Text fontSize="sm" color="fg.muted">
              Sign in to your self-hosted <code>@job-hunter/backend</code> (see <code>docs/backend.md</code>) so the mobile app can read your data even when this desktop is offline. Your password is kept in the OS credential store, never in a file. You can skip this and stay local-only for now, and set it up later in Settings.
            </Text>
            <Field.Root>
              <Input size="sm" placeholder="https://backend.example.com" value={backendUrl} onChange={(e) => setBackendUrl(e.target.value)} />
            </Field.Root>
            <HStack>
              <Input size="sm" type="email" placeholder="Email" value={email} onChange={(e) => setEmail(e.target.value)} />
              <Input size="sm" type="password" placeholder="Password (min 8 characters)" value={password} onChange={(e) => setPassword(e.target.value)} />
            </HStack>
            <HStack>
              <Button
                size="sm"
                colorPalette="brand"
                onClick={() => register.mutate({ backendUrl: backendUrl.trim(), email: email.trim(), password })}
                loading={register.isPending}
                disabled={!backendUrl.trim() || !email.trim() || password.length < 8}
              >
                Create account &amp; connect
              </Button>
              <Button size="sm" variant="ghost" onClick={() => setBackendSkipped(true)}>
                Skip for now
              </Button>
            </HStack>
            <Text fontSize="xs" color="fg.muted">
              Already have an account? Sign in from Settings → Backend account instead.
            </Text>
          </VStack>
        )}
      </Step>

      <Step n={5} title="Import master resume" ok={resumeOk}>
        <HStack>
          <Text fontSize="sm" color="fg.muted" flex="1">
            {resumeOk ? "Master resume imported." : "Import your current PDF or DOCX resume. Claude can then pre-fill your profile from it."}
          </Text>
          <Button size="sm" variant={resumeOk ? "outline" : "solid"} colorPalette="brand" onClick={pick} loading={importResume.isPending}>
            {resumeOk ? "Replace" : "Import Resume"}
          </Button>
          {resumeOk ? (
            <Button size="sm" variant="subtle" onClick={() => parse.mutate()} loading={parse.isPending}>
              Fill profile with Claude
            </Button>
          ) : null}
        </HStack>
      </Step>

      <Step n={6} title="Configure candidate profile" ok={profileOk} warn={!profileOk && resumeOk}>
        <HStack>
          <Text fontSize="sm" color="fg.muted" flex="1">
            {profileOk ? "Profile is ready." : `Still needed: ${s.profile.missing.join(", ")}.`}
          </Text>
          <Button size="sm" variant="outline" onClick={() => navigate({ to: "/candidate" })}>
            Open profile <ChevronRight size={14} />
          </Button>
        </HStack>
      </Step>

      <Box pt={6}>
        {allReady ? (
          <VStack align="flex-start" gap={3}>
            <Heading size="lg">You&apos;re ready.</Heading>
            <HStack>
              <Button colorPalette="brand" size="lg" onClick={() => finish(true)} loading={saveSettings.isPending || start.isPending}>
                Start First Job Hunt
              </Button>
              <Button variant="ghost" onClick={() => finish(false)}>
                Go to dashboard
              </Button>
            </HStack>
          </VStack>
        ) : (
          <HStack>
            <Text fontSize="sm" color="fg.muted" flex="1">
              Finish the highlighted steps to start a job hunt. You can always come back here from Settings.
            </Text>
            <Button variant="ghost" onClick={() => finish(false)} disabled={saveSettings.isPending}>
              Continue to dashboard anyway
            </Button>
          </HStack>
        )}
      </Box>
    </Box>
  );
}
