import type { PendingQuestion } from "@job-hunter/types";

import type { QuestionAnswer } from "./api";

/** The input to show for a question's `fieldType` (anything unknown is plain text). */
export type QuestionInput = "text" | "textarea" | "number" | "date" | "select" | "radio";

export function questionInput(q: Pick<PendingQuestion, "fieldType" | "options">): QuestionInput {
  const type = q.fieldType.trim().toLowerCase();
  if (type === "select" || type === "radio") return q.options.length > 0 ? type : "text";
  if (type === "textarea" || type === "number" || type === "date") return type;
  return "text";
}

/**
 * The answers to send for a form of pending questions (keyed by question
 * id), and the ids of required questions still unanswered. Blank optional
 * answers are left out, as the desktop does.
 */
export function collectAnswers(
  questions: PendingQuestion[],
  values: Record<string, string>,
): { answers: QuestionAnswer[]; missing: string[] } {
  const answers: QuestionAnswer[] = [];
  const missing: string[] = [];
  for (const q of questions) {
    const answer = (values[q.id] ?? "").trim();
    if (answer) answers.push({ question: q.question, answer });
    else if (q.required) missing.push(q.id);
  }
  return { answers, missing };
}
