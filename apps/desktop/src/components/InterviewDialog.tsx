import { Field, Input, NativeSelect, Textarea, VStack } from "@chakra-ui/react";
import { INTERVIEW_MODE_LABELS } from "@job-hunter/shared";
import type { InterviewMode } from "@job-hunter/types";
import { useState } from "react";
import { ConfirmDialog, ErrorBanner } from "./common";
import { useRecordInterview } from "../lib/queries";

/** "Got an interview call": record when, how and what to prepare. */
export function InterviewDialog({ open, onOpenChange, applicationId, company }: { open: boolean; onOpenChange: (open: boolean) => void; applicationId: string; company: string }) {
  const record = useRecordInterview();
  const [when, setWhen] = useState("");
  const [mode, setMode] = useState<InterviewMode | "">("");
  const [round, setRound] = useState("");
  const [contact, setContact] = useState("");
  const [notes, setNotes] = useState("");

  const reset = () => {
    setWhen("");
    setMode("");
    setRound("");
    setContact("");
    setNotes("");
    record.reset();
  };

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={(next) => {
        if (!next) reset();
        onOpenChange(next);
      }}
      title="Got an interview call?"
      description={`Record the interview ${company} invited you to. Everything is optional; add what you know.`}
      confirmLabel="Record interview"
      loading={record.isPending}
      onConfirm={() =>
        record.mutate(
          {
            id: applicationId,
            interview: {
              scheduledAt: when ? new Date(when).toISOString() : null,
              mode,
              round: round.trim(),
              contact: contact.trim(),
              notes: notes.trim(),
            },
          },
          {
            onSuccess: () => {
              reset();
              onOpenChange(false);
            },
          },
        )
      }
    >
      <VStack align="stretch" gap={3} mt={3}>
        <ErrorBanner error={record.error} />
        <Field.Root>
          <Field.Label>When</Field.Label>
          <Input type="datetime-local" value={when} onChange={(e) => setWhen(e.target.value)} />
        </Field.Root>
        <Field.Root>
          <Field.Label>How</Field.Label>
          <NativeSelect.Root size="sm">
            <NativeSelect.Field value={mode} onChange={(e) => setMode(e.target.value as InterviewMode | "")}>
              <option value="">Not sure yet</option>
              {Object.entries(INTERVIEW_MODE_LABELS).map(([value, label]) => (
                <option key={value} value={value}>
                  {label}
                </option>
              ))}
            </NativeSelect.Field>
            <NativeSelect.Indicator />
          </NativeSelect.Root>
        </Field.Root>
        <Field.Root>
          <Field.Label>Round</Field.Label>
          <Input value={round} onChange={(e) => setRound(e.target.value)} placeholder="e.g. HR screening, Technical round 1" />
        </Field.Root>
        <Field.Root>
          <Field.Label>Who called</Field.Label>
          <Input value={contact} onChange={(e) => setContact(e.target.value)} placeholder="Name or team" />
        </Field.Root>
        <Field.Root>
          <Field.Label>Notes</Field.Label>
          <Textarea value={notes} onChange={(e) => setNotes(e.target.value)} placeholder="What they said, what to prepare" rows={3} />
        </Field.Root>
      </VStack>
    </ConfirmDialog>
  );
}
