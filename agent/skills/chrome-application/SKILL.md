# Skill: chrome-application

Complete one **approved** job application in the user's browser through Claude in Chrome.

You are operating a real user's browser.

Never submit an application unless the application record is explicitly approved.

Do not bypass CAPTCHA.

Do not attempt to defeat anti-bot protections.

If a CAPTCHA or blocking mechanism appears: stop and request manual action.

Never claim success without visible evidence of successful submission.

## Inputs

- `application`: id, `approved` flag (must be `true`), job URL, company, title.
- `candidate`: contact details and profile facts that may be typed into forms.
- `documents`: absolute file paths of the approved resume (PDF and/or DOCX) and cover letter.
- `answers`: known question/answer pairs. Only these, the candidate data and the documents may be entered.
- `previouslyAnswered` (on resume): answers the user just provided for questions you reported.

## Procedure

1. Confirm `approved` is true. If not, return `FAILED` with reason "application not approved" and do nothing.
2. `tabs_context_mcp` (`createIfEmpty: true`), open a new tab, navigate to the job URL.
3. Locate the application entry point (Apply, Easy Apply, Apply on company site). Follow it. If it leads to an external applicant tracking system, continue there.
4. Fill the form step by step:
   - contact fields from `candidate`;
   - resume upload with `file_upload` using the given path (prefer PDF unless the form requires DOCX);
   - cover letter upload or paste when the form has a field for it;
   - each question: use `answers` (match by meaning), then candidate data. If no truthful answer exists, **do not guess** – collect the question, field type, options and whether it is required.
5. If any questions were collected, stop **before submitting**, and return `HUMAN_INPUT_REQUIRED` with `unknownQuestions`. Leave the tab open so the session can be resumed.
6. When every required field is filled and the record is approved, submit. Then read the page and capture the confirmation text.
7. Return `SUBMITTED` only when the page (or an on-screen banner/message) clearly confirms the application was received. Quote it in `evidence` and include the final `applicationUrl`.
8. Otherwise return `MANUAL_ACTION_REQUIRED` (CAPTCHA, login, "verify you are human", website error, unsupported multi-step assessment, file upload failure, posting closed, account creation required, payment) or `FAILED` (unexpected error), with a plain-language `reason` and `stepsCompleted`.

## Never

- create accounts, agree to paid services, or change account settings;
- apply to a different job than the one given;
- enter information that is not in the inputs;
- retry a CAPTCHA or solve puzzles;
- report success without evidence.

## Output

Only the JSON structure defined by the apply schema.
