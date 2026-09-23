import { Box, Button, Checkbox, Grid, GridItem, Heading, HStack, Input, NativeSelect, SimpleGrid, Spinner, Tabs, Text, Textarea, VStack } from "@chakra-ui/react";
import { useNavigate, useParams } from "@tanstack/react-router";
import { applicationActions, formatDateTime, isAgentRunning } from "@job-hunter/shared";
import type { ApplicationAnswer, ApplicationStatus, PendingQuestion, ResumeDocument } from "@job-hunter/types";
import { KeyValue, MatchScore, StatusBadge } from "@job-hunter/ui";
import { ArrowLeft, Check, ExternalLink, Pencil, Play, RefreshCw, Send, X } from "lucide-react";
import { useEffect, useState } from "react";
import { BulletList, Chips, ConfirmDialog, ErrorBanner, InfoBanner } from "../components/common";
import { FileLinks, HtmlPreview } from "../components/DocumentPreview";
import { ResumeEditor } from "../components/ResumeEditor";
import { ActivityFeed } from "../components/ActivityFeed";
import { useActivity } from "../lib/activity";
import { useAgentStatus, useAnswerQuestions, useApplication, useApplyApplication, useApproveApplication, useGenerateResume, useMarkManualComplete, useOpenUrl, useRejectApplication, useSetApplicationStatus, useUpdateCoverLetter, useUpdateResumeContent, useSettings } from "../lib/queries";

export function ApplicationReviewPage() {
  const { applicationId } = useParams({ from: "/applications/$applicationId" });
  const detail = useApplication(applicationId);
  const agent = useAgentStatus();
  const settings = useSettings();
  const approve = useApproveApplication();
  const reject = useRejectApplication();
  const apply = useApplyApplication();
  const answer = useAnswerQuestions();
  const markApplied = useMarkManualComplete();
  const setStatus = useSetApplicationStatus();
  const regenerate = useGenerateResume();
  const updateResume = useUpdateResumeContent();
  const updateCover = useUpdateCoverLetter();
  const openUrl = useOpenUrl();
  const navigate = useNavigate();
  const activity = useActivity();

  const [confirm, setConfirm] = useState<"approve" | "reject" | "applied" | null>(null);
  const [rejectReason, setRejectReason] = useState("");
  const [editingResume, setEditingResume] = useState(false);
  const [editingCover, setEditingCover] = useState(false);
  const [coverDraft, setCoverDraft] = useState("");
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const [saveForReuse, setSaveForReuse] = useState(true);

  useEffect(() => {
    if (detail.data?.coverLetter) setCoverDraft(detail.data.coverLetter.text);
  }, [detail.data?.coverLetter]);

  if (detail.isLoading) return <Spinner />;
  if (detail.error || !detail.data) return <ErrorBanner error={detail.error ?? "Application not found"} onRetry={() => detail.refetch()} />;
  const { application, job, analysis, resume, coverLetter } = detail.data;
  const busy = agent.data ? isAgentRunning(agent.data.state) : false;
  const thisRunActive = busy && agent.data?.runKind === "APPLICATION" && activity.some((e) => e.runId === agent.data?.runId);
  const actions = applicationActions(application.status, busy);
  const applyUrl = application.applicationUrl || job.url;
  const runEvents = activity.filter((e) => e.runId === application.runId || (agent.data?.runId && e.runId === agent.data.runId && agent.data.runKind === "APPLICATION"));

  const submitAnswers = () => {
    const list: ApplicationAnswer[] = application.pendingQuestions.map((q) => ({ question: q.question, answer: (answers[q.id] ?? "").trim(), source: "USER" }));
    if (list.some((a, i) => !a.answer && application.pendingQuestions[i]?.required)) return;
    answer.mutate({ id: application.id, answers: list.filter((a) => a.answer), saveForReuse });
  };

  return (
    <Box>
      <HStack mb={4} justify="space-between">
        <Button variant="ghost" size="sm" onClick={() => navigate({ to: "/applications" })}>
          <ArrowLeft size={16} /> Applications
        </Button>
        <HStack gap={2}>
          <Button size="sm" variant="outline" onClick={() => openUrl.mutate(applyUrl)}>
            <ExternalLink size={14} /> {actions.canApplyManually ? "Apply Manually" : "Open Job Website"}
          </Button>
          {actions.canMarkApplied ? (
            <Button size="sm" variant="outline" colorPalette="green" onClick={() => setConfirm("applied")}>
              <Check size={14} /> Mark as Applied
            </Button>
          ) : null}
          {actions.canReject ? (
            <Button size="sm" variant="outline" colorPalette="red" onClick={() => setConfirm("reject")}>
              <X size={14} /> Reject
            </Button>
          ) : null}
          {actions.canApply ? (
            <Button size="sm" colorPalette="brand" onClick={() => apply.mutate({ id: application.id })} loading={apply.isPending}>
              <Play size={14} /> Apply now
            </Button>
          ) : null}
          {actions.canApprove ? (
            <Button size="sm" colorPalette="brand" onClick={() => setConfirm("approve")} disabled={busy}>
              <Check size={14} /> Approve &amp; Apply
            </Button>
          ) : null}
        </HStack>
      </HStack>
      <ErrorBanner error={approve.error ?? reject.error ?? apply.error ?? answer.error ?? markApplied.error ?? setStatus.error ?? regenerate.error ?? updateResume.error ?? updateCover.error} />

      <Box borderWidth="1px" borderColor="border.muted" borderRadius="md" bg="bg.panel" overflow="hidden">
        <Box px={6} py={5} borderBottomWidth="1px" borderColor="border.muted">
          <HStack justify="space-between" align="flex-start">
            <Box>
              <Heading size="xl">{job.title}</Heading>
              <Text fontSize="lg" color="fg.muted">
                {job.company}
                {job.location ? ` · ${job.location}` : ""}
                {job.remote ? ` · ${job.remote}` : ""}
              </Text>
            </Box>
            <VStack align="flex-end" gap={1}>
              <StatusBadge status={application.status} size="md" />
              <Text fontSize="xs" color="fg.muted">
                Updated {formatDateTime(application.updatedAt)}
              </Text>
            </VStack>
          </HStack>
        </Box>

        {application.status === "WAITING_FOR_USER" && application.pendingQuestions.length > 0 ? (
          <Box px={6} py={5} borderBottomWidth="1px" borderColor="border.muted" bg="orange.subtle">
            <Heading size="sm" mb={1}>
              Human input required
            </Heading>
            <Text fontSize="sm" color="fg.muted" mb={4}>
              {application.failureReason ?? "The application form asks questions the agent could not answer truthfully. Answer them below to continue."}
            </Text>
            <VStack align="stretch" gap={3} maxW="720px">
              {application.pendingQuestions.map((q) => (
                <QuestionField key={q.id} q={q} value={answers[q.id] ?? ""} onChange={(v) => setAnswers((a) => ({ ...a, [q.id]: v }))} />
              ))}
              <HStack justify="space-between">
                <Checkbox.Root checked={saveForReuse} onCheckedChange={(e) => setSaveForReuse(!!e.checked)} size="sm">
                  <Checkbox.HiddenInput />
                  <Checkbox.Control />
                  <Checkbox.Label>Remember these answers for future applications</Checkbox.Label>
                </Checkbox.Root>
                <Button colorPalette="brand" onClick={submitAnswers} loading={answer.isPending} disabled={busy}>
                  <Send size={14} /> Continue application
                </Button>
              </HStack>
            </VStack>
          </Box>
        ) : null}

        {application.status === "MANUAL_ACTION_REQUIRED" ? (
          <Box px={6} py={5} borderBottomWidth="1px" borderColor="border.muted" bg="orange.subtle">
            <Heading size="sm" mb={1}>
              Application could not be completed automatically
            </Heading>
            <Text fontSize="sm" mb={3}>
              <Text as="span" fontWeight="semibold">
                Reason:
              </Text>{" "}
              {application.failureReason ?? "Unknown"}
            </Text>
            <Text fontSize="sm" color="fg.muted" mb={3}>
              The application has been prepared. Open it in Chrome, finish it with the documents below, then mark it as applied.
            </Text>
            <HStack>
              <Button size="sm" colorPalette="brand" onClick={() => openUrl.mutate(applyUrl)}>
                <ExternalLink size={14} /> Open Application
              </Button>
              <Button size="sm" variant="outline" onClick={() => setConfirm("applied")}>
                <Check size={14} /> Mark as Applied
              </Button>
              {application.approvedAt ? (
                <Button size="sm" variant="ghost" onClick={() => apply.mutate({ id: application.id })} loading={apply.isPending} disabled={busy}>
                  <RefreshCw size={14} /> Retry automatically
                </Button>
              ) : null}
            </HStack>
          </Box>
        ) : null}

        {application.status === "APPLIED" ? (
          <Box px={6} py={4} borderBottomWidth="1px" borderColor="border.muted" bg="green.subtle">
            <Text fontSize="sm">
              <Text as="span" fontWeight="semibold">
                Applied {formatDateTime(application.appliedAt)}
              </Text>
              {application.manualCompleted ? " (completed manually)" : ""}
              {application.evidence ? ` · Evidence: ${application.evidence}` : ""}
            </Text>
            <HStack mt={2} gap={2}>
              <Text fontSize="sm" color="fg.muted">
                Track:
              </Text>
              {(["INTERVIEW", "OFFER", "REJECTED", "WITHDRAWN"] as ApplicationStatus[]).map((s) => (
                <Button key={s} size="xs" variant="outline" onClick={() => setStatus.mutate({ id: application.id, status: s })} loading={setStatus.isPending}>
                  {s.charAt(0) + s.slice(1).toLowerCase()}
                </Button>
              ))}
            </HStack>
          </Box>
        ) : null}

        {(application.status === "INTERVIEW" || application.status === "OFFER") ? (
          <Box px={6} py={4} borderBottomWidth="1px" borderColor="border.muted" bg="green.subtle">
            <HStack gap={2}>
              <Text fontSize="sm" fontWeight="semibold">
                {application.status === "INTERVIEW" ? "Interview stage" : "Offer received"}
              </Text>
              {(["OFFER", "REJECTED", "WITHDRAWN"] as ApplicationStatus[]).filter((s) => s !== application.status).map((s) => (
                <Button key={s} size="xs" variant="outline" onClick={() => setStatus.mutate({ id: application.id, status: s })}>
                  {s.charAt(0) + s.slice(1).toLowerCase()}
                </Button>
              ))}
            </HStack>
          </Box>
        ) : null}

        {thisRunActive ? (
          <Box px={6} py={4} borderBottomWidth="1px" borderColor="border.muted">
            <Heading size="xs" mb={2} color="fg.muted" textTransform="uppercase" letterSpacing="wide">
              Claude is working in Chrome
            </Heading>
            <ActivityFeed events={runEvents.slice(-30)} maxHeight="200px" compact />
          </Box>
        ) : null}

        <Box px={6} py={5} borderBottomWidth="1px" borderColor="border.muted">
          <Heading size="xs" mb={2} color="fg.muted" textTransform="uppercase" letterSpacing="wide">
            Job summary
          </Heading>
          <Text fontSize="sm" className="selectable" whiteSpace="pre-wrap" lineHeight="1.6" maxH="220px" overflowY="auto">
            {job.description || "No description captured."}
          </Text>
          {analysis?.summary ? (
            <Text fontSize="sm" mt={3} fontStyle="italic" color="fg.muted">
              {analysis.summary}
            </Text>
          ) : null}
        </Box>

        <Grid templateColumns={{ base: "1fr", lg: "2fr 3fr" }}>
          <GridItem px={6} py={5} borderRightWidth={{ lg: "1px" }} borderBottomWidth={{ base: "1px", lg: 0 }} borderColor="border.muted">
            <Heading size="xs" mb={3} color="fg.muted" textTransform="uppercase" letterSpacing="wide">
              Match analysis
            </Heading>
            {analysis ? (
              <VStack align="stretch" gap={4}>
                <MatchScore score={analysis.matchScore} relevant={analysis.relevant} />
                <KeyValue label="Skills">
                  <Chips items={analysis.matchedSkills} palette="green" max={15} />
                  {analysis.missingSkills.length ? (
                    <Box mt={1.5}>
                      <Chips items={analysis.missingSkills} palette="orange" max={15} />
                    </Box>
                  ) : null}
                </KeyValue>
                <KeyValue label="Experience">{analysis.requiredExperienceMet ? "Requirement met" : "Posting asks for more experience than your records show"}</KeyValue>
                <KeyValue label="Location">{analysis.locationMatch ? "Matches your preferences" : "Outside your preferences"}</KeyValue>
                <KeyValue label="Concerns">
                  <BulletList items={[...analysis.concerns, ...application.potentialIssues.filter((i) => !analysis.concerns.includes(i))]} />
                </KeyValue>
              </VStack>
            ) : (
              <Text fontSize="sm" color="fg.muted">
                No analysis.
              </Text>
            )}
          </GridItem>
          <GridItem px={6} py={5}>
            <HStack justify="space-between" mb={3}>
              <Heading size="xs" color="fg.muted" textTransform="uppercase" letterSpacing="wide">
                Application materials
              </Heading>
              <HStack gap={2}>
                <FileLinks resume={resume} coverLetter={coverLetter} />
                <Button size="xs" variant="ghost" onClick={() => regenerate.mutate(job.id)} loading={regenerate.isPending} disabled={busy || application.status === "APPLIED"}>
                  <RefreshCw size={12} /> Regenerate
                </Button>
              </HStack>
            </HStack>
            <Tabs.Root defaultValue="resume" size="sm" variant="line">
              <Tabs.List mb={3}>
                <Tabs.Trigger value="resume">Resume{resume?.validation ? ` · ATS ${resume.validation.status}` : ""}</Tabs.Trigger>
                <Tabs.Trigger value="cover" disabled={!coverLetter}>
                  Cover letter
                </Tabs.Trigger>
                <Tabs.Trigger value="answers">Answers ({application.answers.length})</Tabs.Trigger>
                <Tabs.Trigger value="history">History</Tabs.Trigger>
              </Tabs.List>
              <Tabs.Content value="resume">
                {resume ? (
                  editingResume && resume.content ? (
                    <ResumeEditor
                      value={resume.content}
                      saving={updateResume.isPending}
                      onCancel={() => setEditingResume(false)}
                      onSave={(content: ResumeDocument) => updateResume.mutate({ id: resume.id, content }, { onSuccess: () => setEditingResume(false) })}
                    />
                  ) : (
                    <VStack align="stretch" gap={3}>
                      {resume.validation ? (
                        <SimpleGrid columns={3} gap={2} fontSize="xs">
                          <KeyValue label="Keyword coverage">{resume.validation.keywordCoverage}%</KeyValue>
                          <KeyValue label="Unsupported claims">{resume.validation.unsupportedClaims.length === 0 ? "None" : resume.validation.unsupportedClaims.join("; ")}</KeyValue>
                          <KeyValue label="Missing keywords">{resume.validation.missingKeywords.length === 0 ? "None" : resume.validation.missingKeywords.join(", ")}</KeyValue>
                        </SimpleGrid>
                      ) : null}
                      {resume.validation?.unsupportedClaims.length ? (
                        <InfoBanner status="warning" title="Review before applying">
                          The validator flagged statements it could not verify against your profile. Edit the resume to remove anything that is not true.
                        </InfoBanner>
                      ) : null}
                      <HStack>
                        <Button size="xs" variant="outline" onClick={() => setEditingResume(true)} disabled={!resume.content || application.status === "APPLIED"}>
                          <Pencil size={12} /> Edit Resume
                        </Button>
                        <Text fontSize="xs" color="fg.muted">
                          {resume.label}
                          {resume.userEdited ? " · edited by you" : ""}
                        </Text>
                      </HStack>
                      <HtmlPreview path={resume.htmlPath} height="640px" />
                    </VStack>
                  )
                ) : (
                  <Text fontSize="sm" color="fg.muted">
                    No resume generated yet.
                  </Text>
                )}
              </Tabs.Content>
              <Tabs.Content value="cover">
                {coverLetter ? (
                  editingCover ? (
                    <VStack align="stretch" gap={2}>
                      <Textarea rows={16} value={coverDraft} onChange={(e) => setCoverDraft(e.target.value)} fontSize="sm" />
                      <HStack justify="flex-end">
                        <Button size="sm" variant="ghost" onClick={() => { setEditingCover(false); setCoverDraft(coverLetter.text); }}>
                          Cancel
                        </Button>
                        <Button size="sm" colorPalette="brand" loading={updateCover.isPending} onClick={() => updateCover.mutate({ id: coverLetter.id, text: coverDraft }, { onSuccess: () => setEditingCover(false) })}>
                          Save cover letter
                        </Button>
                      </HStack>
                    </VStack>
                  ) : (
                    <VStack align="stretch" gap={2}>
                      <HStack>
                        <Button size="xs" variant="outline" onClick={() => setEditingCover(true)} disabled={application.status === "APPLIED"}>
                          <Pencil size={12} /> Edit Cover Letter
                        </Button>
                        {coverLetter.userEdited ? (
                          <Text fontSize="xs" color="fg.muted">
                            edited by you
                          </Text>
                        ) : null}
                      </HStack>
                      <Box p={4} borderWidth="1px" borderColor="border.muted" borderRadius="md" fontSize="sm" whiteSpace="pre-wrap" lineHeight="1.7" className="selectable">
                        {coverLetter.text}
                      </Box>
                    </VStack>
                  )
                ) : null}
              </Tabs.Content>
              <Tabs.Content value="answers">
                {application.answers.length === 0 ? (
                  <Text fontSize="sm" color="fg.muted">
                    No answers yet.
                  </Text>
                ) : (
                  <VStack align="stretch" gap={2}>
                    {application.answers.map((a) => (
                      <HStack key={a.question} justify="space-between" fontSize="sm" borderBottomWidth="1px" borderColor="border.muted" py={1.5}>
                        <Text fontWeight="medium">{a.question}</Text>
                        <Text color="fg.muted" textAlign="right">
                          {a.answer} <Text as="span" fontSize="xs">({a.source.toLowerCase()})</Text>
                        </Text>
                      </HStack>
                    ))}
                  </VStack>
                )}
              </Tabs.Content>
              <Tabs.Content value="history">
                <VStack align="stretch" gap={1.5} fontSize="sm">
                  {[...application.statusHistory].reverse().map((h, i) => (
                    <HStack key={i} justify="space-between">
                      <HStack>
                        <StatusBadge status={h.status} size="xs" />
                        <Text color="fg.muted">{h.reason}</Text>
                      </HStack>
                      <Text color="fg.subtle" fontSize="xs">
                        {formatDateTime(h.at)}
                      </Text>
                    </HStack>
                  ))}
                  {application.notes ? (
                    <Box mt={3} p={3} bg="bg.muted" borderRadius="md" whiteSpace="pre-wrap" className="selectable" fontSize="xs">
                      {application.notes}
                    </Box>
                  ) : null}
                </VStack>
              </Tabs.Content>
            </Tabs.Root>
          </GridItem>
        </Grid>

        {actions.canApprove ? (
          <HStack px={6} py={4} borderTopWidth="1px" borderColor="border.muted" justify="space-between" bg="bg.subtle">
            <Button variant="outline" colorPalette="red" onClick={() => setConfirm("reject")}>
              <X size={16} /> Reject
            </Button>
            <HStack gap={3}>
              {settings.data?.mockMode ? (
                <Text fontSize="xs" color="orange.fg">
                  Mock mode: nothing will be submitted
                </Text>
              ) : null}
              <Button colorPalette="brand" size="lg" onClick={() => setConfirm("approve")} disabled={busy}>
                <Check size={16} /> Approve &amp; Apply
              </Button>
            </HStack>
          </HStack>
        ) : null}
      </Box>

      <ConfirmDialog
        open={confirm === "approve"}
        onOpenChange={(o) => !o && setConfirm(null)}
        title="Approve and apply?"
        description={`Job Hunter will open ${job.company}'s application in Chrome and fill it with this resume${coverLetter ? " and cover letter" : ""}. It only submits after every required field is filled, and stops to ask you about anything it cannot answer.`}
        confirmLabel="Approve & Apply"
        loading={approve.isPending}
        onConfirm={() => approve.mutate({ id: application.id, applyNow: true }, { onSuccess: () => setConfirm(null) })}
      />
      <ConfirmDialog open={confirm === "reject"} onOpenChange={(o) => !o && setConfirm(null)} title="Reject this application?" destructive confirmLabel="Reject" loading={reject.isPending} onConfirm={() => reject.mutate({ id: application.id, reason: rejectReason }, { onSuccess: () => setConfirm(null) })}>
        <Input mt={3} placeholder="Reason (optional)" value={rejectReason} onChange={(e) => setRejectReason(e.target.value)} />
      </ConfirmDialog>
      <ConfirmDialog open={confirm === "applied"} onOpenChange={(o) => !o && setConfirm(null)} title="Mark as applied?" description="Only confirm this after you submitted the application yourself." confirmLabel="Mark as Applied" loading={markApplied.isPending} onConfirm={() => markApplied.mutate({ id: application.id }, { onSuccess: () => setConfirm(null) })} />
    </Box>
  );
}

function QuestionField({ q, value, onChange }: { q: PendingQuestion; value: string; onChange: (v: string) => void }) {
  const label = (
    <Text fontSize="sm" fontWeight="medium" mb={1}>
      {q.question}
      {q.required ? (
        <Text as="span" color="red.fg">
          {" "}
          *
        </Text>
      ) : null}
      {q.context ? (
        <Text as="span" color="fg.muted" fontWeight="normal">
          {" "}
          — {q.context}
        </Text>
      ) : null}
    </Text>
  );
  if ((q.fieldType === "select" || q.fieldType === "radio") && q.options.length) {
    return (
      <Box>
        {label}
        <NativeSelect.Root size="sm">
          <NativeSelect.Field value={value} onChange={(e) => onChange(e.target.value)}>
            <option value="">Choose…</option>
            {q.options.map((o) => (
              <option key={o} value={o}>
                {o}
              </option>
            ))}
          </NativeSelect.Field>
          <NativeSelect.Indicator />
        </NativeSelect.Root>
      </Box>
    );
  }
  if (q.fieldType === "textarea") {
    return (
      <Box>
        {label}
        <Textarea size="sm" rows={3} value={value} onChange={(e) => onChange(e.target.value)} />
      </Box>
    );
  }
  return (
    <Box>
      {label}
      <Input size="sm" type={q.fieldType === "number" ? "number" : q.fieldType === "date" ? "date" : "text"} value={value} onChange={(e) => onChange(e.target.value)} />
    </Box>
  );
}
