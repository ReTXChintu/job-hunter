# Skill: candidate-profile

Parse an imported master resume into structured candidate information.

## Inputs

- `resumeText`: the extracted text of the user's PDF/DOCX resume.
- `existingProfile` (optional): what the user already entered; do not overwrite non-empty user values.

## Rules

- Extract only what the document states. Leave fields empty when the resume does not mention them.
- Dates as `YYYY-MM` (or `YYYY` when only the year is given). A current position has `endDate: null`.
- Group skills into frontend, backend, database, devops, cloud, testing, other; put a skill in exactly one group.
- Each employer becomes one `experiences` entry with `technologies` and `achievements` taken from its bullets.
- Named projects become `projects` entries; link them to an employer via `experienceCompany` when the resume makes that clear.
- Copy education, certifications, achievements and languages verbatim.
- Never add information to make the profile look better.

## Output

Only the JSON structure defined by the profile-parse schema.
