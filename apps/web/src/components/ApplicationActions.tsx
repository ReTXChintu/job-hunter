import { Alert, Button, Field, Flex, Input, Text, VStack } from "@chakra-ui/react";
import { STATUS_LABELS } from "@job-hunter/shared";
import type { ApplicationListItem, ApplicationStatus } from "@job-hunter/types";
import { Check, RefreshCw, X } from "lucide-react";
import { useState, type ReactNode } from "react";

import { useDesktopAction } from "../desktopActions";
import { useSession } from "../session";
import { trackingStatuses } from "../statusGroups";
import { ConfirmDialog } from "./ConfirmDialog";
import { QuestionForm } from "./QuestionForm";
import { ErrorBox, Panel } from "./State";

type Dialog = { kind: "approve" } | { kind: "reject" } | { kind: "applied" } | { kind: "status"; status: ApplicationStatus } | null;

const STATUS_HINTS: Partial<Record<ApplicationStatus, string>> = {
  INTERVIEW: "You've been invited to interview.",
  OFFER: "You've received an offer.",
  REJECTED: "The employer turned the application down.",
  WITHDRAWN: "You withdrew the application.",
};

/**
 * What the user can do about an application from the web. Every action is
 * a request to the desktop, which runs it through the same approval-gated
 * code as its own buttons; nothing here is shown as done until it replies.
 */
export function ApplicationActions({ item }: { item: ApplicationListItem }) {
  const { api } = useSession();
  const { application: app, job } = item;
  const action = useDesktopAction([["application", app.id], ["applications"], ["notifications"]]);
  const [dialog, setDialog] = useState<Dialog>(null);
  const [reason, setReason] = useState("");
  const [note, setNote] = useState("");

  const byEmail = !!job.applyEmail;
  const tracking = trackingStatuses(app.status);
  const inDialog = dialog !== null;

  const open = (next: Dialog) => {
    action.reset();
    setReason("");
    setNote("");
    setDialog(next);
  };
  const close = () => {
    setDialog(null);
    action.reset();
  };
  const closeOnSuccess = { onSuccess: () => setDialog(null) };

  const confirm = () => {
    if (!dialog) return;
    switch (dialog.kind) {
      case "approve":
        action.run(
          {
            key: "approve",
            run: () => api.approveApplication(app.id),
            success: byEmail ? "Approved. Your desktop is sending the email now." : "Approved. Your desktop is applying now.",
          },
          closeOnSuccess,
        );
        break;
      case "reject":
        action.run({ key: "reject", run: () => api.rejectApplication(app.id, reason.trim()), success: "Application rejected." }, closeOnSuccess);
        break;
      case "applied":
        action.run({ key: "applied", run: () => api.markApplicationApplied(app.id, note.trim()), success: "Marked as applied." }, closeOnSuccess);
        break;
      case "status":
        action.run(
          {
            key: "status",
            run: () => api.setApplicationStatus(app.id, dialog.status, note.trim()),
            success: `Marked as ${STATUS_LABELS[dialog.status].toLowerCase()}.`,
          },
          closeOnSuccess,
        );
        break;
    }
  };

  const noteField = (
    <Field.Root mt={4}>
      <Field.Label>Note (optional)</Field.Label>
      <Input value={note} onChange={(e) => setNote(e.target.value)} disabled={action.busy} />
    </Field.Root>
  );

  let body: ReactNode = null;
  switch (app.status) {
    case "WAITING_FOR_USER":
      body = (
        <Panel title="Questions waiting for your answer">
          {app.pendingQuestions.length > 0 ? (
            <VStack align="stretch" gap={4}>
              <Text fontSize="sm" color="fg.muted">
                The application form asked something only you can answer. Your desktop continues the application once you send these.
              </Text>
              <QuestionForm
                key={app.pendingQuestions.map((q) => q.id).join("|")}
                questions={app.pendingQuestions}
                busy={action.busy}
                onSubmit={(answers) =>
                  action.run({
                    key: "answer",
                    run: () => api.answerApplicationQuestions(app.id, answers),
                    success: "Sent to your desktop; it continues the application.",
                  })
                }
              />
            </VStack>
          ) : (
            <Text fontSize="sm" color="fg.muted">
              The application is waiting for you, but no questions came through. Open it in the desktop app.
            </Text>
          )}
        </Panel>
      );
      break;
    case "READY_FOR_REVIEW":
      body = (
        <Panel title="Your approval">
          <Text fontSize="sm" color="fg.muted" mb={4}>
            {byEmail
              ? `This posting asks for applications by email. Approving sends one email to ${job.applyEmail} from your Gmail.`
              : "Nothing is sent until you approve. Approving has your desktop fill in and submit the application."}
          </Text>
          <Flex gap={2} wrap="wrap">
            <Button colorPalette="brand" size="sm" onClick={() => open({ kind: "approve" })} disabled={action.busy}>
              <Check size={14} /> {byEmail ? "Approve & send email" : "Approve & apply"}
            </Button>
            <Button colorPalette="red" variant="outline" size="sm" onClick={() => open({ kind: "reject" })} disabled={action.busy}>
              <X size={14} /> Reject
            </Button>
          </Flex>
        </Panel>
      );
      break;
    case "MANUAL_ACTION_REQUIRED":
      body = (
        <Panel title="Finish this application">
          <Text fontSize="sm" color="fg.muted" mb={4}>
            Your desktop couldn&apos;t complete this on its own. Let it try again, or finish it yourself from the posting and mark it as applied.
          </Text>
          <Flex gap={2} wrap="wrap">
            {app.approvedAt ? (
              <Button
                colorPalette="brand"
                size="sm"
                onClick={() =>
                  action.run({ key: "retry", run: () => api.applyApplication(app.id), success: "Your desktop is trying the application again." })
                }
                loading={action.running("retry")}
                loadingText="Waiting for your desktop…"
                disabled={action.busy}
              >
                <RefreshCw size={14} /> {app.claudeSessionId ? "Resume on desktop" : "Retry on desktop"}
              </Button>
            ) : null}
            <Button colorPalette="green" variant="outline" size="sm" onClick={() => open({ kind: "applied" })} disabled={action.busy}>
              <Check size={14} /> Mark as applied
            </Button>
          </Flex>
        </Panel>
      );
      break;
    default:
      if (tracking.length > 0) {
        body = (
          <Panel title="Track progress">
            <Text fontSize="sm" color="fg.muted" mb={4}>
              Heard back from {job.company}? Record where the application stands.
            </Text>
            <Flex gap={2} wrap="wrap">
              {tracking.map((status) => (
                <Button
                  key={status}
                  size="sm"
                  variant="outline"
                  colorPalette={status === "REJECTED" || status === "WITHDRAWN" ? "red" : "green"}
                  onClick={() => open({ kind: "status", status })}
                  disabled={action.busy}
                >
                  {STATUS_LABELS[status]}
                </Button>
              ))}
            </Flex>
          </Panel>
        );
      }
  }

  return (
    <VStack align="stretch" gap={3}>
      {action.success ? (
        <Alert.Root status="success" borderRadius="md" role="status" aria-live="polite">
          <Alert.Indicator />
          <Alert.Content>
            <Alert.Description>{action.success}</Alert.Description>
          </Alert.Content>
        </Alert.Root>
      ) : null}
      {action.error && !inDialog ? <ErrorBox error={action.error} title="Your desktop couldn't do that" /> : null}
      {body}

      <ConfirmDialog
        open={dialog?.kind === "approve"}
        onClose={close}
        title={byEmail ? "Approve and send the email?" : "Approve and apply?"}
        description={
          byEmail
            ? `Your desktop opens Gmail in Chrome and sends one email to ${job.applyEmail} with your tailored resume attached. It never emails anyone else.`
            : `Your desktop opens ${job.company}'s application in Chrome and fills it in with your tailored resume. It submits only once every required field is filled, and asks you about anything it can't answer.`
        }
        confirmLabel={byEmail ? "Approve & send email" : "Approve & apply"}
        loading={action.running("approve")}
        error={action.error}
        onConfirm={confirm}
      />
      <ConfirmDialog
        open={dialog?.kind === "reject"}
        onClose={close}
        title="Reject this application?"
        description="Nothing will be sent to the employer."
        confirmLabel="Reject"
        destructive
        loading={action.running("reject")}
        error={action.error}
        onConfirm={confirm}
      >
        <Field.Root mt={4}>
          <Field.Label>Reason (optional)</Field.Label>
          <Input value={reason} onChange={(e) => setReason(e.target.value)} disabled={action.busy} placeholder="e.g. salary too low" />
        </Field.Root>
      </ConfirmDialog>
      <ConfirmDialog
        open={dialog?.kind === "applied"}
        onClose={close}
        title="Mark as applied?"
        description="Only confirm this once you've submitted the application yourself."
        confirmLabel="Mark as applied"
        loading={action.running("applied")}
        error={action.error}
        onConfirm={confirm}
      >
        {noteField}
      </ConfirmDialog>
      <ConfirmDialog
        open={dialog?.kind === "status"}
        onClose={close}
        title={dialog?.kind === "status" ? `Mark as ${STATUS_LABELS[dialog.status].toLowerCase()}?` : ""}
        description={dialog?.kind === "status" ? STATUS_HINTS[dialog.status] : undefined}
        confirmLabel={dialog?.kind === "status" ? `Mark as ${STATUS_LABELS[dialog.status].toLowerCase()}` : "Confirm"}
        destructive={dialog?.kind === "status" && (dialog.status === "REJECTED" || dialog.status === "WITHDRAWN")}
        loading={action.running("status")}
        error={action.error}
        onConfirm={confirm}
      >
        {noteField}
      </ConfirmDialog>
    </VStack>
  );
}
