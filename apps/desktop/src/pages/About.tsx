import { Box, Button, HStack, SimpleGrid, Spinner, Text, VStack } from "@chakra-ui/react";
import { formatDateTime } from "@job-hunter/shared";
import { Briefcase, RefreshCw } from "lucide-react";

import { ErrorBanner, InfoBanner, PageHeader, Panel } from "../components/common";
import { UpdateActions } from "../components/UpdateNotifier";
import { useAboutInfo, useUpdateCheck } from "../lib/queries";

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <Box>
      <Text fontSize="xs" color="fg.muted">
        {label}
      </Text>
      <Text fontSize="sm" fontFamily="mono" wordBreak="break-all">
        {children}
      </Text>
    </Box>
  );
}

export function AboutPage() {
  const about = useAboutInfo();
  const check = useUpdateCheck();
  const a = about.data;
  const c = check.data;

  return (
    <Box maxW="760px">
      <PageHeader title="About" />
      <VStack align="stretch" gap={4}>
        <Panel>
          <HStack gap={4}>
            <Box bg="brand.solid" color="brand.contrast" borderRadius="lg" p={3} display="flex">
              <Briefcase size={28} />
            </Box>
            <Box>
              <Text fontSize="xl" fontWeight="semibold">
                Job Hunter
              </Text>
              <Text color="fg.muted">{a ? `Version ${a.version}` : <Spinner size="xs" />}</Text>
            </Box>
          </HStack>
        </Panel>

        <Panel
          title="Updates"
          action={
            <Button size="xs" variant="outline" onClick={() => void check.refetch()} loading={check.isFetching}>
              <RefreshCw size={12} /> Check for updates
            </Button>
          }
        >
          {check.error ? <ErrorBanner error={check.error} title="Couldn't check for updates" onRetry={() => void check.refetch()} /> : null}
          {c ? (
            c.available ? (
              <VStack align="stretch" gap={3}>
                <Text fontSize="sm">
                  <Text as="span" fontWeight="semibold">
                    Version {c.latestVersion} is available.
                  </Text>{" "}
                  You have {c.currentVersion}.{c.publishedAt ? ` Published ${formatDateTime(c.publishedAt)}.` : ""}
                </Text>
                <UpdateActions check={c} />
                {!c.canInstallInApp ? (
                  <Text fontSize="xs" color="fg.muted">
                    Run the downloaded installer to update. It keeps all your data.
                  </Text>
                ) : null}
              </VStack>
            ) : (
              <InfoBanner status="success">
                You&apos;re up to date{c.latestVersion ? ` (latest is ${c.latestVersion})` : ""}.
              </InfoBanner>
            )
          ) : check.isPending ? (
            <Spinner size="sm" />
          ) : null}
        </Panel>

        <Panel title="Details">
          {a ? (
            <SimpleGrid columns={{ base: 1, md: 2 }} gap={4}>
              <Row label="Version">{a.version}</Row>
              <Row label="Platform">{a.platform}</Row>
              <Row label="Server">{a.serverUrl}{a.serverBuiltIn ? "" : " (development default)"}</Row>
              <Row label="In-app updates">{a.updaterEnabled ? "Enabled" : "Not available in this build"}</Row>
              <Box gridColumn={{ md: "span 2" }}>
                <Row label="Data folder">{a.dataDir}</Row>
              </Box>
            </SimpleGrid>
          ) : (
            <Spinner size="sm" />
          )}
        </Panel>
      </VStack>
    </Box>
  );
}
