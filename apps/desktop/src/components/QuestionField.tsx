import { Box, Input, NativeSelect, Text, Textarea } from "@chakra-ui/react";
import type { PendingQuestion } from "@job-hunter/types";

/** One question a website asked that only the user can answer. */
export function QuestionField({ q, value, onChange }: { q: PendingQuestion; value: string; onChange: (v: string) => void }) {
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
