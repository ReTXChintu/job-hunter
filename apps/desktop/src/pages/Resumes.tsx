import { Box, Button, Grid, GridItem, HStack, Table, Text } from "@chakra-ui/react";
import { Link } from "@tanstack/react-router";
import { formatDateTime } from "@job-hunter/shared";
import { EmptyState } from "@job-hunter/ui";
import { ExternalLink } from "lucide-react";
import { useState } from "react";
import { ErrorBanner, PageHeader, Panel } from "../components/common";
import { HtmlPreview } from "../components/DocumentPreview";
import { MasterResumeTab } from "./Candidate";
import { useOpenPath, useProfile, useResumes } from "../lib/queries";

export function ResumesPage() {
  const resumes = useResumes();
  const profile = useProfile();
  const open = useOpenPath();
  const [selected, setSelected] = useState<string | null>(null);
  const list = resumes.data ?? [];
  const current = list.find((r) => r.id === selected) ?? list[0];

  return (
    <Box>
      <PageHeader title="Resumes" subtitle="Your master resume and every tailored version the agent generated, grouped per company." />
      <ErrorBanner error={resumes.error} onRetry={() => resumes.refetch()} />
      {profile.data ? (
        <Box mb={6}>
          <MasterResumeTab profile={profile.data} compact />
        </Box>
      ) : null}
      {list.length === 0 ? (
        <EmptyState title="No generated resumes yet" description="Tailored resumes appear here once the agent prepares applications." />
      ) : (
        <Grid templateColumns={{ base: "1fr", xl: "1fr 1fr" }} gap={6}>
          <GridItem>
            <Panel p={0} title="Generated versions">
              <Table.Root size="sm" interactive>
                <Table.Header>
                  <Table.Row>
                    <Table.ColumnHeader>Company</Table.ColumnHeader>
                    <Table.ColumnHeader>Version</Table.ColumnHeader>
                    <Table.ColumnHeader>ATS</Table.ColumnHeader>
                    <Table.ColumnHeader>Created</Table.ColumnHeader>
                    <Table.ColumnHeader></Table.ColumnHeader>
                  </Table.Row>
                </Table.Header>
                <Table.Body>
                  {list.map((r) => (
                    <Table.Row key={r.id} cursor="pointer" bg={current?.id === r.id ? "bg.muted" : undefined} onClick={() => setSelected(r.id)}>
                      <Table.Cell>
                        <Text fontWeight="medium">{r.company}</Text>
                        <Text fontSize="xs" color="fg.muted">
                          {r.jobTitle}
                        </Text>
                      </Table.Cell>
                      <Table.Cell>v{r.version}{r.userEdited ? " (edited)" : ""}</Table.Cell>
                      <Table.Cell>
                        {r.validation ? (
                          <Text fontSize="sm" color={r.validation.status === "PASS" ? "green.fg" : "orange.fg"}>
                            {r.validation.status} · {r.validation.keywordCoverage}%
                          </Text>
                        ) : (
                          "—"
                        )}
                      </Table.Cell>
                      <Table.Cell>
                        <Text fontSize="xs" color="fg.muted">
                          {formatDateTime(r.createdAt)}
                        </Text>
                      </Table.Cell>
                      <Table.Cell>
                        <HStack gap={1}>
                          {r.pdfPath ? (
                            <Button size="2xs" variant="ghost" onClick={(e) => { e.stopPropagation(); open.mutate(r.pdfPath!); }}>
                              PDF
                            </Button>
                          ) : null}
                          {r.docxPath ? (
                            <Button size="2xs" variant="ghost" onClick={(e) => { e.stopPropagation(); open.mutate(r.docxPath!); }}>
                              DOCX
                            </Button>
                          ) : null}
                          {r.jobId ? (
                            <Link to="/jobs/$jobId" params={{ jobId: r.jobId }} onClick={(e) => e.stopPropagation()}>
                              <Button size="2xs" variant="ghost">
                                <ExternalLink size={10} /> Job
                              </Button>
                            </Link>
                          ) : null}
                        </HStack>
                      </Table.Cell>
                    </Table.Row>
                  ))}
                </Table.Body>
              </Table.Root>
            </Panel>
          </GridItem>
          <GridItem>{current ? <HtmlPreview path={current.htmlPath} height="760px" /> : null}</GridItem>
        </Grid>
      )}
    </Box>
  );
}
