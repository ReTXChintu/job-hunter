# Skill: inbox-tracking

Find employers' replies to the candidate's job applications in their Gmail,
in the **Inbox and the Spam folder**, through Claude in Chrome. This task is
read-only.

You are operating a real user's browser, signed in to their real Gmail.

## Inputs

- `applications`: what the candidate applied to: `applicationId`, `company`,
  `title`, `applyEmail` (set when the application was sent by email),
  `appliedAt`, `source`, and `knownReplies` (already found; don't report them
  again).
- `sinceDays`: how far back to look.

## Procedure

1. `tabs_context_mcp` (`createIfEmpty: true`), open one new tab and go to
   `https://mail.google.com/mail/u/0/#inbox`. If Gmail shows a sign-in page,
   return `blocked: true` with the reason.
2. Search Gmail with its search box / search URL
   (`https://mail.google.com/mail/u/0/#search/<query>`), a few applications
   per query, always including spam and excluding what the candidate sent:
   `in:anywhere -in:sent -in:drafts newer_than:<sinceDays>d ("Company A" OR from:hr@companya.com OR "Company B")`.
   Also run `in:spam newer_than:<sinceDays>d` once and scan it for anything
   about a job application, interview or assessment. Job-site messages
   (Naukri, LinkedIn, Indeed, Wellfound, Cutshort, Instahyre, Hirist,
   Foundit) that are about one of the listed applications count too:
   recruiter messages, "application viewed/shortlisted", interview invites.
3. Open each candidate message and read enough to be sure which application
   it is about and what it asks. Skip newsletters, job alerts, marketing,
   generic "jobs you may like" and messages not tied to one of the listed
   applications.
4. For each real reply return `applicationId`, `from` (name and address as
   shown), `subject`, `receivedAt` (as Gmail shows the date and time),
   `folder` ("INBOX" or "SPAM"), `kind` and `summary`:
   - `INTERVIEW`: an interview or call is being scheduled or proposed;
   - `ASSESSMENT`: a test, assignment or coding challenge;
   - `QUESTION`: they ask the candidate for something (details, documents,
     availability, expected salary);
   - `OFFER`;
   - `REJECTION`;
   - `ACKNOWLEDGEMENT`: received / under review / viewed / shortlisted with
     no action needed;
   - `OTHER`.
   `summary`: one or two sentences with what they want and any deadline,
   date or link the candidate needs.

## Never

- reply, forward, send, delete, archive, label, mark as spam/not spam, or
  change any setting (opening a message to read it is fine);
- report a message that isn't clearly about one of the listed applications;
- repeat one listed in `knownReplies`.

## Output

Only the JSON structure defined by the inbox-check schema.
