# Skill: shared-job-reading

The candidate found a job themselves and shared it: pasted text, and/or
screenshots and PDFs (a LinkedIn post, a WhatsApp forward, a job
description document). Turn it into one structured job.

## Inputs

- `text`: what the candidate pasted (may be empty).
- `files`: absolute paths of the screenshots/PDFs they shared (may be
  empty). Open every one with the Read tool; it shows images and PDFs.

## Rules

- Read everything before answering; a screenshot may continue the text.
- Use only what the material says. Never invent a company, title, salary,
  location, email or requirement. Leave a field empty when it isn't there.
- `title`: the role as written. `company`: the hiring company (not a
  recruiter's agency name when the client is named; the poster's company
  if that's all there is).
- `applyEmail`: the address the posting asks candidates to send resumes to,
  exactly as written (fix only obvious OCR slips such as a space inside the
  address). Empty if there is none.
- `contactName`: the person to address, if the posting names one.
- `url`: a link to the posting if one is visible; otherwise empty.
- `description`: the full posting text, transcribed from screenshots where
  needed. `requirements`, `responsibilities`, `skills`: as lists.
- If the material isn't a job posting (or is unreadable), return
  `found: false` with the reason.

## Output

Only the JSON structure defined by the shared-job schema.
