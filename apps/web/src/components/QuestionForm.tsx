import { Button, Field, Fieldset, Input, NativeSelect, RadioGroup, Text, Textarea, VStack } from "@chakra-ui/react";
import type { PendingQuestion } from "@job-hunter/types";
import { Send } from "lucide-react";
import { useState, type FormEvent } from "react";

import type { QuestionAnswer } from "../api";
import { collectAnswers, questionInput } from "../questions";

const controlId = (q: PendingQuestion) => `question-${q.id}`;

function QuestionControl({ q, value, onChange }: { q: PendingQuestion; value: string; onChange: (v: string) => void }) {
  switch (questionInput(q)) {
    case "select":
      return (
        <NativeSelect.Root size="sm">
          <NativeSelect.Field id={controlId(q)} value={value} onChange={(e) => onChange(e.target.value)}>
            <option value="">Choose…</option>
            {q.options.map((o) => (
              <option key={o} value={o}>
                {o}
              </option>
            ))}
          </NativeSelect.Field>
          <NativeSelect.Indicator />
        </NativeSelect.Root>
      );
    case "textarea":
      return <Textarea id={controlId(q)} size="sm" rows={3} value={value} onChange={(e) => onChange(e.target.value)} />;
    case "number":
      return <Input id={controlId(q)} size="sm" type="number" inputMode="decimal" value={value} onChange={(e) => onChange(e.target.value)} />;
    case "date":
      return <Input id={controlId(q)} size="sm" type="date" value={value} onChange={(e) => onChange(e.target.value)} />;
    default:
      return <Input id={controlId(q)} size="sm" type="text" value={value} onChange={(e) => onChange(e.target.value)} />;
  }
}

function Question({ q, value, invalid, disabled, onChange }: { q: PendingQuestion; value: string; invalid: boolean; disabled: boolean; onChange: (v: string) => void }) {
  if (questionInput(q) === "radio") {
    // A group of choices is a fieldset, with the question as its legend.
    return (
      <Fieldset.Root invalid={invalid} disabled={disabled} id={controlId(q)} tabIndex={-1}>
        <Fieldset.Legend fontSize="sm" fontWeight="medium">
          {q.question}
          {q.required ? (
            <Text as="span" color="fg.error" aria-hidden>
              {" "}
              *
            </Text>
          ) : null}
        </Fieldset.Legend>
        {q.context ? <Fieldset.HelperText>{q.context}</Fieldset.HelperText> : null}
        <RadioGroup.Root size="sm" value={value || null} onValueChange={(e) => onChange(e.value ?? "")} aria-required={q.required || undefined}>
          <VStack align="start" gap={2}>
            {q.options.map((o) => (
              <RadioGroup.Item key={o} value={o}>
                <RadioGroup.ItemHiddenInput />
                <RadioGroup.ItemIndicator />
                <RadioGroup.ItemText>{o}</RadioGroup.ItemText>
              </RadioGroup.Item>
            ))}
          </VStack>
        </RadioGroup.Root>
        {invalid ? <Fieldset.ErrorText>Choose an answer.</Fieldset.ErrorText> : null}
      </Fieldset.Root>
    );
  }
  return (
    <Field.Root required={q.required} invalid={invalid} disabled={disabled} ids={{ control: controlId(q) }}>
      <Field.Label fontSize="sm">
        {q.question}
        <Field.RequiredIndicator />
      </Field.Label>
      <QuestionControl q={q} value={value} onChange={onChange} />
      {q.context ? <Field.HelperText>{q.context}</Field.HelperText> : null}
      {invalid ? <Field.ErrorText>This answer is required.</Field.ErrorText> : null}
    </Field.Root>
  );
}

/**
 * The questions an application form asked that only the user can answer.
 * Answers are sent only when every required one is filled in.
 */
export function QuestionForm({
  questions,
  busy,
  onSubmit,
}: {
  questions: PendingQuestion[];
  busy: boolean;
  onSubmit: (answers: QuestionAnswer[]) => void;
}) {
  const [values, setValues] = useState<Record<string, string>>({});
  const [showErrors, setShowErrors] = useState(false);
  const { answers, missing } = collectAnswers(questions, values);

  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (busy) return;
    if (missing.length > 0) {
      setShowErrors(true);
      document.getElementById(`question-${missing[0]}`)?.focus();
      return;
    }
    onSubmit(answers);
  };

  return (
    <form onSubmit={submit} noValidate>
      <VStack align="stretch" gap={4}>
        {questions.map((q) => (
          <Question
            key={q.id}
            q={q}
            value={values[q.id] ?? ""}
            invalid={showErrors && missing.includes(q.id)}
            disabled={busy}
            onChange={(v) => setValues((prev) => ({ ...prev, [q.id]: v }))}
          />
        ))}
        {showErrors && missing.length > 0 ? (
          <Text fontSize="sm" color="fg.error" role="alert">
            Answer the required {missing.length === 1 ? "question" : `${missing.length} questions`} marked * first.
          </Text>
        ) : null}
        <Button type="submit" colorPalette="brand" alignSelf="flex-start" loading={busy} loadingText="Sending to your desktop…">
          <Send size={14} /> Send answers and continue
        </Button>
        <Text fontSize="xs" color="fg.muted">
          Your answers are also saved on your desktop for future applications.
        </Text>
      </VStack>
    </form>
  );
}
