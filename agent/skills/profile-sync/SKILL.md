# Skill: profile-sync

Bring the candidate's **own profile** on one job site in line with their
Job Hunter profile, through Claude in Chrome. The candidate asked for this
and confirmed the content; the task states `CONFIRMED = true`.

You are operating a real user's browser, signed in to their real account.

Do not bypass CAPTCHA.

Do not attempt to defeat anti-bot protections.

## Inputs

- `platform`: LinkedIn, Naukri, Indeed or Wellfound.
- `profile`: the content to publish: personal details, `headline`, `summary`,
  `skills`, `experiences`, `projects` (only the ones the candidate chose),
  `education`, `certifications`, `languages`, `careerPreferences`.
- `removeProjects`: names of projects Job Hunter published earlier that the
  candidate no longer features. Remove exactly these from the site.
- `knownAnswers`: question/answer pairs the candidate saved. Use them for
  site fields not covered by `profile` (current CTC, date of birth, ...).
- `previouslyAnswered`: answers the candidate just gave to questions you
  reported last time.
- `resumePath`: the candidate's master resume file, if any.

## Procedure

1. `tabs_context_mcp` (`createIfEmpty: true`), open **one** new tab, go to
   the site and open the signed-in user's own profile page:
   - LinkedIn: `https://www.linkedin.com/in/me/`
   - Naukri: `https://www.naukri.com/mnjuser/profile`
   - Indeed: `https://profile.indeed.com/`
   - Wellfound: `https://wellfound.com/profile/edit/overview`
2. If the site shows a login page, CAPTCHA, OTP/verification, or "unusual
   activity": stop and return `MANUAL_ACTION_REQUIRED` with the reason. Don't
   try to sign in.
3. Read the profile as it is now. For each section the site has, compare it
   with `profile` and edit only what differs:
   - **Headline / title / summary ("About", "Profile summary").**
   - **Skills**: add missing ones from `profile.skills`. Respect the site's
     limit by adding the most relevant first (matching `targetRoles`). Don't
     remove skills the candidate added themselves.
   - **Experience / employment**: one entry per `experiences` item, matched by
     company and role. Update title, dates, location and description; add
     missing entries. Put technologies and achievements into the description
     when the site has no field for them.
   - **Projects**: add or update each item of `projects` (name, description,
     role, technologies, the employer it was done at, URL). Remove the ones
     in `removeProjects`. Leave every other project on the site alone.
   - **Education, certifications, languages.**
   - **Career preferences** where the site has them (Naukri's desired
     role/location, expected salary, notice period; Indeed/Wellfound job
     preferences) from `careerPreferences`.
   - **Resume**: upload `resumePath` with `file_upload` only when the site
     has no resume at all.
   Save each section with the site's own Save button and check it saved.
4. When a field the site **requires** for saving has no truthful value in
   `profile`, `knownAnswers` or `previouslyAnswered`, don't guess. Finish the
   other sections, then return `HUMAN_INPUT_REQUIRED` listing each such field
   in `unknownQuestions` (question as the site words it, field type, options,
   whether required).
5. Return `UPDATED` when everything the site supports now matches `profile`.
   List what you changed in `changes` ("Added skill Next.js", "Updated
   headline", "Added project Sort-A-Snap"), sections the site has no place
   for or that already matched in `skipped`, the names of `profile.projects`
   now on the site in `projectsOnSite`, and the profile URL in `profileUrl`.

## Never

- post, share, or publish an update to the feed. On LinkedIn, switch off
  "Share with network" / "Notify network" before saving when the toggle is
  shown;
- send messages, connection requests, follows, or endorsements;
- apply to jobs, change job alerts, open-to-work visibility, privacy or
  account settings, email, phone, or password;
- delete anything except the projects in `removeProjects`;
- enter information that is not in the inputs, or change a fact to something
  the inputs don't state;
- report a change you didn't see saved.

## Output

Only the JSON structure defined by the profile-sync schema.
