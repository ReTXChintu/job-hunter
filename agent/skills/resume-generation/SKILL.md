# Skill: resume-generation

Produce a tailored, ATS-friendly resume (as structured content) and, when requested, a cover letter for one job.

## Inputs

- `candidate`: profile, skill groups, experience records, projects, education, certifications, achievements, languages, and the master resume text (source of truth).
- `job`: the posting with description, requirements, responsibilities and skills.
- `analysis`: matched/missing skills and important keywords.
- `previousValidation` (optional): feedback from the validator on a prior attempt; fix every point it raises.

## Rules

1. **Truth only.** Every line must be supported by the candidate data. If the master resume and the structured profile disagree, prefer the structured profile and never combine them into something that is not true.
2. Keep the candidate's real job titles and employers. Do not rename a title to match the posting.
3. `headline`: the candidate's real current title or a truthful descriptor (e.g. "Full Stack Developer (React, Node.js)").
4. `summary`: three to four sentences targeted at this job, naming the technologies the candidate actually has that the posting asks for.
5. `skills`: grouped sections (e.g. Frontend, Backend, Databases, Cloud & DevOps, Testing, Other). Order the groups and items so the posting's keywords appear early, but include only skills the candidate listed.
6. `experience`: most relevant first if the dates allow (otherwise reverse-chronological). Three to six bullets each; start with a verb; include technologies and any real metrics from the records. Do not add metrics that are not in the records.
7. `projects`: pick the two to four most relevant; each with a one-line description and technologies.
8. `education`, `certifications`, `achievements`, `languages`: copy from the records.
9. No tables, columns, images, icons or unusual characters. Plain text only.
10. Cover letter (when requested): under 300 words, addressed to the hiring team at the company, mentioning the role, two or three concrete relevant experiences, and why the candidate is interested based on the posting only. No claims about the company beyond the posting. End with the candidate's name.
11. `applicationNotes`: short list of things the user should know when applying (a stated requirement the candidate does not meet, a question the form may ask, etc.).

## Output

Only the JSON structure defined by the resume schema.
