import { Box, Button, Grid, GridItem, Heading, HStack, SimpleGrid, Spinner, Tabs, Text, VStack } from "@chakra-ui/react";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { formatDateTime, isAgentRunning } from "@job-hunter/shared";
import { KeyValue, MatchScore, StatusBadge } from "@job-hunter/ui";
import { ArrowLeft, Bookmark, BookmarkCheck, Check, ExternalLink, FileText, RefreshCw, Sparkles, X } from "lucide-react";
import { useState } from "react";
import { BulletList, Chips, ConfirmDialog, ErrorBanner, InfoBanner, Panel } from "../components/common";
import { FileLinks, HtmlPreview } from "../components/DocumentPreview";
import { useAgentStatus, useAnalyzeJob, useApproveApplication, useGenerateResume, useJob, useOpenUrl, useRejectJob, useSetJobSaved } from "../lib/queries";

export function JobDetailPage() {
  const { jobId } = useParams({ from: "/jobs/$jobId" });
  const detail = useJob(jobId);
  const agent = useAgentStatus();
  const analyze = useAnalyzeJob();
  const generate = useGenerateResume();
  const approve = useApproveApplication();
  const reject = useRejectJob();
  const save = useSetJobSaved();
  const openUrl = useOpenUrl();
  const navigate = useNavigate();
  const [confirmApprove, setConfirmApprove] = useState(false);
  const busy = agent.data ? isAgentRunning(agent.data.state) : false;

  if (detail.isLoading) return <Spinner />;
  if (detail.error || !detail.data) return <ErrorBanner error={detail.error ?? "Job not found"} onRetry={() => detail.refetch()} />;
  const { job, analysis, application, resume, coverLetter } = detail.data;
  const canApprove = application?.status === "READY_FOR_REVIEW";
  const hasMaterials = !!resume;

  return (
    <Box>
      <HStack mb={4} justify="space-between">
        <Button variant="ghost" size="sm" onClick={() => navigate({ to: "/jobs" })}>
          <ArrowLeft size={16} /> Jobs
        </Button>
        <HStack gap={2}>
          <Button size="sm" variant="outline" onClick={() => openUrl.mutate(job.url)}>
            <ExternalLink size={14} /> Open Original Job
          </Button>
          <Button size="sm" variant="outline" onClick={() => save.mutate({ id: job.id, saved: !job.saved })}>
            {job.saved ? <BookmarkCheck size={14} /> : <Bookmark size={14} />} {job.saved ? "Saved" : "Save"}
          </Button>
          {job.status !== "REJECTED" && job.status !== "APPLIED" ? (
            <Button size="sm" variant="outline" colorPalette="red" onClick={() => reject.mutate(job.id)} loading={reject.isPending}>
              <X size={14} /> Reject
            </Button>
          ) : null}
          {canApprove ? (
            <Button size="sm" colorPalette="brand" onClick={() => setConfirmApprove(true)} disabled={busy}>
              <Check size={14} /> Approve &amp; Apply
            </Button>
          ) : application ? (
            <Button size="sm" colorPalette="brand" variant="subtle" onClick={() => navigate({ to: "/applications/$applicationId", params: { applicationId: application.id } })}>
              Open application
            </Button>
          ) : (
            <Button size="sm" colorPalette="brand" onClick={() => generate.mutate(job.id)} loading={generate.isPending} disabled={busy || job.status === "REJECTED"}>
              <Sparkles size={14} /> Prepare application
            </Button>
          )}
        </HStack>
      </HStack>
      <ErrorBanner error={analyze.error ?? generate.error ?? approve.error ?? reject.error} />

      <Box mb={5}>
        <HStack gap={3} mb={1}>
          <Heading size="xl">{job.title}</Heading>
          <StatusBadge status={job.status} size="md" />
        </HStack>
        <Text fontSize="lg" color="fg.muted">
          {job.company}
          {job.location ? ` · ${job.location}` : ""}
          {job.remote ? ` · ${job.remote}` : ""}
        </Text>
      </Box>

      <SimpleGrid columns={{ base: 2, md: 5 }} gap={4} mb={6}>
        <KeyValue label="Salary">{job.salary ?? "Not stated"}</KeyValue>
        <KeyValue label="Posted">{job.postedAt ?? "Unknown"}</KeyValue>
        <KeyValue label="Employment">{job.employmentType ?? "Not stated"}</KeyValue>
        <KeyValue label="Seniority">{job.seniority ?? "Not stated"}</KeyValue>
        <KeyValue label="Sources">
          <VStack align="flex-start" gap={0.5}>
            {job.sources.map((s) => (
              <Button key={s.url} variant="plain" size="xs" px={0} onClick={() => openUrl.mutate(s.url)}>
                {s.platform} <ExternalLink size={10} />
              </Button>
            ))}
          </VStack>
        </KeyValue>
      </SimpleGrid>

      <Grid templateColumns={{ base: "1fr", xl: "3fr 2fr" }} gap={6}>
        <GridItem>
          <Tabs.Root defaultValue="description" variant="line" size="sm">
            <Tabs.List mb={3}>
              <Tabs.Trigger value="description">Job description</Tabs.Trigger>
              <Tabs.Trigger value="resume" disabled={!hasMaterials}>
                Generated resume
              </Tabs.Trigger>
              <Tabs.Trigger value="cover" disabled={!coverLetter}>
                Cover letter
              </Tabs.Trigger>
              <Tabs.Trigger value="answers" disabled={!application}>
                Application answers
              </Tabs.Trigger>
            </Tabs.List>
            <Tabs.Content value="description">
              <Panel>
                <Text whiteSpace="pre-wrap" fontSize="sm" className="selectable" lineHeight="1.6">
                  {job.description || "The full description was not captured. Use Open Original Job to read it on the source site."}
                </Text>
                {job.responsibilities.length ? (
                  <Box mt={5}>
                    <Heading size="sm" mb={2}>
                      Responsibilities
                    </Heading>
                    <BulletList items={job.responsibilities} />
                  </Box>
                ) : null}
                {job.requirements.length ? (
                  <Box mt={5}>
                    <Heading size="sm" mb={2}>
                      Requirements
                    </Heading>
                    <BulletList items={job.requirements} />
                  </Box>
                ) : null}
              </Panel>
            </Tabs.Content>
            <Tabs.Content value="resume">
              {resume ? (
                <VStack align="stretch" gap={3}>
                  <HStack justify="space-between">
                    <Text fontSize="sm" color="fg.muted">
                      {resume.label}
                      {resume.validation ? ` · ATS ${resume.validation.status} (${resume.validation.keywordCoverage}% keywords)` : ""}
                    </Text>
                    <FileLinks resume={resume} coverLetter={coverLetter} />
                  </HStack>
                  <HtmlPreview path={resume.htmlPath} />
                </VStack>
              ) : null}
            </Tabs.Content>
            <Tabs.Content value="cover">
              {coverLetter ? (
                <Panel>
                  <Text whiteSpace="pre-wrap" fontSize="sm" className="selectable" lineHeight="1.7">
                    {coverLetter.text}
                  </Text>
                </Panel>
              ) : null}
            </Tabs.Content>
            <Tabs.Content value="answers">
              {application ? (
                <Panel>
                  {application.answers.length === 0 ? (
                    <Text fontSize="sm" color="fg.muted">
                      No answers prepared yet.
                    </Text>
                  ) : (
                    <VStack align="stretch" gap={2}>
                      {application.answers.map((a) => (
                        <Box key={a.question} fontSize="sm">
                          <Text fontWeight="semibold">{a.question}</Text>
                          <Text color="fg.muted">
                            {a.answer} <Text as="span" fontSize="xs">({a.source.toLowerCase()})</Text>
                          </Text>
                        </Box>
                      ))}
                    </VStack>
                  )}
                </Panel>
              ) : null}
            </Tabs.Content>
          </Tabs.Root>
        </GridItem>

        <GridItem>
          <Panel title="Match analysis" mb={4} action={
            <Button size="xs" variant="ghost" onClick={() => analyze.mutate(job.id)} loading={analyze.isPending} disabled={busy}>
              <RefreshCw size={12} /> {analysis ? "Re-analyze" : "Analyze"}
            </Button>
          }>
            {!analysis ? (
              <Text fontSize="sm" color="fg.muted">
                Not analyzed yet.
              </Text>
            ) : (
              <VStack align="stretch" gap={4}>
                <MatchScore score={analysis.matchScore} relevant={analysis.relevant} />
                <Text fontSize="sm" className="selectable">
                  {analysis.summary}
                </Text>
                <SimpleGrid columns={2} gap={2} fontSize="sm">
                  <Flag ok={analysis.requiredExperienceMet} label="Experience" />
                  <Flag ok={analysis.seniorityMatch} label="Seniority" />
                  <Flag ok={analysis.locationMatch} label="Location" />
                  <Flag ok={analysis.employmentTypeMatch} label="Employment type" />
                </SimpleGrid>
                <KeyValue label="Matched skills">
                  <Chips items={analysis.matchedSkills} palette="green" max={20} />
                </KeyValue>
                <KeyValue label="Missing skills">
                  <Chips items={analysis.missingSkills} palette="orange" max={20} />
                </KeyValue>
                <KeyValue label="Important keywords">
                  <Chips items={analysis.importantKeywords} palette="gray" max={20} />
                </KeyValue>
                <KeyValue label="Salary">{analysis.salaryAssessment || "Not stated"}</KeyValue>
                {analysis.concerns.length ? (
                  <KeyValue label="Potential concerns">
                    <BulletList items={analysis.concerns} />
                  </KeyValue>
                ) : null}
              </VStack>
            )}
          </Panel>
          {application ? (
            <Panel title="Application">
              <VStack align="stretch" gap={2} fontSize="sm">
                <HStack justify="space-between">
                  <Text color="fg.muted">Status</Text>
                  <StatusBadge status={application.status} />
                </HStack>
                {application.approvedAt ? (
                  <HStack justify="space-between">
                    <Text color="fg.muted">Approved</Text>
                    <Text>{formatDateTime(application.approvedAt)}</Text>
                  </HStack>
                ) : null}
                {application.appliedAt ? (
                  <HStack justify="space-between">
                    <Text color="fg.muted">Applied</Text>
                    <Text>{formatDateTime(application.appliedAt)}</Text>
                  </HStack>
                ) : null}
                {application.failureReason ? (
                  <InfoBanner status="warning">{application.failureReason}</InfoBanner>
                ) : null}
                <Link to="/applications/$applicationId" params={{ applicationId: application.id }}>
                  <Button size="sm" variant="subtle" w="100%">
                    <FileText size={14} /> Open application review
                  </Button>
                </Link>
              </VStack>
            </Panel>
          ) : null}
        </GridItem>
      </Grid>

      <ConfirmDialog
        open={confirmApprove}
        onOpenChange={setConfirmApprove}
        title="Approve and apply?"
        description={`Job Hunter will open ${job.company}'s application in Chrome and fill it with the generated resume and your known answers. It will stop and ask you if it meets a question it cannot answer, a CAPTCHA, or a login wall.`}
        confirmLabel="Approve & Apply"
        loading={approve.isPending}
        onConfirm={() => {
          if (application) approve.mutate({ id: application.id, applyNow: true }, { onSuccess: () => { setConfirmApprove(false); navigate({ to: "/applications/$applicationId", params: { applicationId: application.id } }); } });
        }}
      />
    </Box>
  );
}

function Flag({ ok, label }: { ok: boolean; label: string }) {
  return (
    <HStack gap={1.5} color={ok ? "green.fg" : "orange.fg"}>
      {ok ? <Check size={14} /> : <X size={14} />}
      <Text color="fg">{label}</Text>
    </HStack>
  );
}
