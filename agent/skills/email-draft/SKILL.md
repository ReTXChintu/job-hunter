# Skill: email-draft

Prepare one job-application email in the candidate's own Gmail **as a
draft**, through Claude in Chrome. The candidate reviews and sends it
themselves. You never send.

You are operating a real user's browser, signed in to their real Gmail.

## Inputs

- `email`: `to` (may be empty when the posting gave no address), `subject`
  and `body`: use them exactly.
- `documents.resumePdf`: absolute path of the resume to attach.

## Procedure

1. `tabs_context_mcp` (`createIfEmpty: true`), open one new tab, go to
   `https://mail.google.com/mail/u/0/#inbox`. If Gmail shows a sign-in page
   or account chooser, return `MANUAL_ACTION_REQUIRED` ("sign in to Gmail in
   Chrome").
2. Click **Compose**. Fill **To** with `email.to` when it isn't empty (press
   Enter/Tab so it becomes a recipient chip), **Subject** with
   `email.subject`, and the body with `email.body`, keeping line breaks.
3. Attach `documents.resumePdf`: use `file_upload` on the compose window's
   hidden file input (`input[type=file]` next to the paper-clip "Attach
   files" button); don't click the paper-clip. Wait until the attachment
   chip finishes uploading.
4. **Do not click Send.** Close the compose window with its X ("Save & close"):
   Gmail keeps it in **Drafts**. Open Drafts and check the draft is there
   with the subject and the attachment.
5. Return `DRAFTED` with `evidence` quoting what Drafts shows. If the
   attachment couldn't be added, still leave the draft and return
   `MANUAL_ACTION_REQUIRED` saying the resume must be attached by hand.

## Never

- click Send, schedule send, or send in any other way;
- email or add anyone other than `email.to`, add CC/BCC, or change the
  subject or body;
- read, open, reply to, archive or delete other emails.

## Output

Only the JSON structure defined by the email-draft schema.
