import { Box, Button, HStack, Input, Text, VStack } from "@chakra-ui/react";
import { COMMON_QUESTIONS, normalizeQuestion } from "@job-hunter/shared";
import type { AnswerRecord } from "@job-hunter/types";
import { Check, Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { ErrorBanner, Panel } from "./common";
import { useAnswers, useDeleteAnswer, useSaveAnswer } from "../lib/queries";

function blankRecord(question: string, answer: string): AnswerRecord {
  const now = new Date().toISOString();
  return { id: "", userId: "", question, normalizedQuestion: "", answer, category: "profile", timesUsed: 0, createdAt: now, updatedAt: now };
}

/**
 * Details job sites and application forms ask for that the rest of the
 * profile doesn't cover (CTC, date of birth, joining date, ...). Answered
 * once here, Claude uses them for every profile update and application
 * instead of stopping to ask.
 */
export function AdditionalDetails() {
  const answers = useAnswers();
  const save = useSaveAnswer();
  const del = useDeleteAnswer();
  const [newQuestion, setNewQuestion] = useState("");
  const [newAnswer, setNewAnswer] = useState("");

  const saved = answers.data ?? [];
  const known = new Set(saved.map((a) => normalizeQuestion(a.question)));
  const suggested = COMMON_QUESTIONS.filter((q) => !known.has(normalizeQuestion(q)));

  return (
    <VStack align="stretch" gap={4}>
      <Text fontSize="sm" color="fg.muted">
        Job sites and application forms ask for details that aren't part of a resume. Answer them once here and Claude uses them everywhere, so profile
        updates and applications don't stop to ask you. Answers you give in the app are saved here too.
      </Text>
      <ErrorBanner error={save.error ?? del.error ?? answers.error} />

      {suggested.length ? (
        <Panel title={`Often asked (${suggested.length} unanswered)`}>
          <VStack align="stretch" gap={2}>
            {suggested.map((q) => (
              <AnswerRow key={q} question={q} initial="" onSave={(answer) => save.mutate(blankRecord(q, answer))} saving={save.isPending} />
            ))}
          </VStack>
        </Panel>
      ) : null}

      <Panel title={`Your answers (${saved.length})`}>
        {saved.length === 0 ? (
          <Text fontSize="sm" color="fg.muted">
            None saved yet.
          </Text>
        ) : (
          <VStack align="stretch" gap={2}>
            {saved.map((a) => (
              <AnswerRow
                key={a.id}
                question={a.question}
                initial={a.answer}
                usedTimes={a.timesUsed}
                onSave={(answer) => save.mutate({ ...a, answer })}
                onDelete={() => del.mutate(a.id)}
                saving={save.isPending}
              />
            ))}
          </VStack>
        )}
        <HStack mt={4} gap={2} align="flex-end">
          <Input size="sm" flex="3" placeholder="Another question a site asks" value={newQuestion} onChange={(e) => setNewQuestion(e.target.value)} />
          <Input size="sm" flex="2" placeholder="Your answer" value={newAnswer} onChange={(e) => setNewAnswer(e.target.value)} />
          <Button
            size="sm"
            variant="subtle"
            disabled={!newQuestion.trim() || !newAnswer.trim()}
            loading={save.isPending}
            onClick={() =>
              save.mutate(blankRecord(newQuestion.trim(), newAnswer.trim()), {
                onSuccess: () => {
                  setNewQuestion("");
                  setNewAnswer("");
                },
              })
            }
          >
            <Plus size={14} /> Add
          </Button>
        </HStack>
      </Panel>
    </VStack>
  );
}

function AnswerRow({ question, initial, usedTimes, onSave, onDelete, saving }: { question: string; initial: string; usedTimes?: number; onSave: (answer: string) => void; onDelete?: () => void; saving: boolean }) {
  const [value, setValue] = useState(initial);
  const changed = value.trim() !== initial.trim() && value.trim().length > 0;
  return (
    <HStack gap={3} align="center">
      <Box flex="3" minW={0}>
        <Text fontSize="sm">{question}</Text>
        {usedTimes ? (
          <Text fontSize="2xs" color="fg.subtle">
            Used {usedTimes} time{usedTimes === 1 ? "" : "s"}
          </Text>
        ) : null}
      </Box>
      <Input size="sm" flex="2" value={value} placeholder="Your answer" onChange={(e) => setValue(e.target.value)} onKeyDown={(e) => e.key === "Enter" && changed && onSave(value.trim())} />
      <Button size="xs" colorPalette="brand" variant={changed ? "solid" : "ghost"} disabled={!changed} loading={saving && changed} onClick={() => onSave(value.trim())} aria-label="Save answer">
        <Check size={12} />
      </Button>
      {onDelete ? (
        <Button size="xs" variant="ghost" colorPalette="red" onClick={onDelete} aria-label="Delete answer">
          <Trash2 size={12} />
        </Button>
      ) : null}
    </HStack>
  );
}
