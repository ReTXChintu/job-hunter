# Skill: resume-validation

Validate a generated resume against the job posting and the candidate's source of truth. You are the reviewer, not the author.

## Inputs

- `candidate`: the source of truth (profile, experience, projects, master resume text).
- `job` and `analysis` (important keywords, required skills).
- `resume`: the generated structured resume.
- `iteration`: which attempt this is (1..3).

## Checks

1. **Unsupported claims** – every skill, technology, title, employer, date, metric, certification, degree and project in the resume must be present in the candidate data. List each unsupported item verbatim. This is the most important check.
2. **Keyword coverage** – percentage of the posting's important keywords / required skills that appear in the resume *and* are supported by the candidate data. Do not penalise missing keywords the candidate genuinely lacks; list them under `missingKeywords` so the user knows.
3. **Job title alignment** – the summary/headline should relate to the target role without misrepresenting the candidate's actual title.
4. **Relevant experience** – the most relevant experience and projects come first and their bullets speak to the posting.
5. **Formatting** – standard headings, no tables/columns/graphics, bullets are concise, no placeholder text, no empty sections.
6. **Missing important requirements** – hard requirements from the posting that the resume does not address even though the candidate data could support them.

## Decision

- `PASS` when there are no unsupported claims, formatting is clean, and keyword coverage is at or above the target given in the task (or all remaining missing keywords are ones the candidate truly lacks).
- `REVISE` when there are fixable issues (unsupported claims, missing supported keywords, ordering, formatting).
- `FAIL` only when the candidate data cannot truthfully support a resume for this job at all.

`notes` must tell the author exactly what to change.

## Output

Only the JSON structure defined by the validation schema.
