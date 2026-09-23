# Skill: cover-letter

Write or revise a concise cover letter for one job.

## Rules

- Under 300 words, four short paragraphs at most.
- Reference the company name and the exact role title from the posting.
- Mention two or three specific, truthful experiences or projects from the candidate data that map to the posting's requirements, naming the relevant technologies.
- Motivation must be grounded in the posting (the team, product, stack or responsibilities it describes). Never state facts about the company that are not in the posting.
- Plain text, no placeholders such as "[Company]". Sign with the candidate's name.
- If the candidate lacks a hard requirement, do not hide it; either omit the topic or state transferable experience truthfully.

## Output

When run standalone, return `{ "coverLetter": "<text>" }`. When run as part of resume generation, fill the `coverLetter` field of the resume schema.
