# Skill: email-application

Send one **approved** application by email from the candidate's own Gmail,
through Claude in Chrome. Used for postings (e.g. LinkedIn hiring posts)
that ask candidates to email their resume instead of filling a form.

You are operating a real user's browser, signed in to their real Gmail.

Never send unless the application record is explicitly approved.

Do not bypass CAPTCHA.

## Inputs

- `application`: id, `approved` flag (must be `true`), the posting URL,
  company, title.
- `email`: `to`, `subject` and `body`: the exact message the candidate
  approved. Send it as given; don't rewrite it.
- `documents.resumePdf`: absolute path of the approved resume to attach.

## Procedure

1. Confirm `approved` is true. If not, return `FAILED` with reason
   "application not approved" and do nothing.
2. `tabs_context_mcp` (`createIfEmpty: true`), open one new tab and go to
   `https://mail.google.com/mail/u/0/#inbox`. If Gmail shows a sign-in page
   or account chooser you can't resolve with the already signed-in account,
   return `MANUAL_ACTION_REQUIRED` ("sign in to Gmail in Chrome").
3. Click **Compose**. Fill **To** with `email.to` (press Enter or Tab so it
   becomes a recipient chip), **Subject** with `email.subject`, and the
   message body with `email.body`, keeping its line breaks.
4. Attach `documents.resumePdf`: find the compose window's hidden file input
   (`input[type=file]`, next to the paper-clip "Attach files" button) with
   `find`/`read_page` and use `file_upload` on it. Don't click the
   paper-clip itself (it opens a system dialog you can't use). Wait until the
   attachment chip with the file name shows and finishes uploading.
5. Check the draft once: recipient, subject, body, attachment. Then click
   **Send**.
6. Return `SUBMITTED` only when Gmail shows "Message sent" (quote it in
   `evidence`), with `applicationUrl` set to the posting URL.
7. If the attachment can't be added, don't send without it: close nothing,
   leave the draft (Gmail saves it) and return `MANUAL_ACTION_REQUIRED`
   explaining that the draft is waiting in Gmail Drafts without the resume.

## Never

- email anyone other than `email.to`, or add CC/BCC;
- change the approved subject or body beyond fixing a paste that lost line
  breaks;
- read, open, reply to, archive or delete other emails;
- report success without Gmail's "Message sent" confirmation.

## Output

Only the JSON structure defined by the apply schema.
