# Skill: job-discovery

Find recent job postings on one job source using Claude in Chrome.

## Inputs

- `source`: one of LinkedIn, LinkedIn Posts, Naukri, Indeed, Wellfound, Cutshort, Instahyre, Hirist, Foundit, Hiring Cafe, Welcome to the Jungle, Himalayas, Y Combinator, Greenhouse, Lever, Workday, or a company careers URL.
- `queries`: search phrases derived from the candidate's target roles.
- `locations`, `remotePreference`, `recencyDays`, `maxJobs`.
- Optional `seenUrls`: postings already known; skip them.

## Procedure

1. Call `tabs_context_mcp` with `createIfEmpty: true`, then open **one** new tab with `tabs_create_mcp` and work in it.
2. Navigate to the search URL for the source (recipes below). Prefer URL parameters over typing into search boxes.
3. Run **every** query in `queries`, not just the first few. Read the results with `get_page_text` (or `read_page` when structure matters). Scroll or paginate only as needed to collect up to `maxJobs` postings in total within the recency window, spreading them across the queries rather than filling the quota from the first one.
4. For each candidate posting that matches the queries, open the posting (same tab or the results detail pane) and extract the full description, requirements, responsibilities and skills. Set `detailsComplete: true` only when the full description was read.
5. Skip postings older than `recencyDays`, postings already in `seenUrls`, and obviously irrelevant roles (different discipline, wrong seniority by title such as "Director" or "Intern" when not targeted).
   - A posting is relevant when its title matches **any** query or a common synonym of one: "Backend Engineer" and "Backend Developer" are the same role, "SDE" means Software Development Engineer / Software Engineer, "Node.js Developer" matches "Node Js. Developer". Do not treat a role as off-target just because another query already produced results.
   - "Different discipline" means things like sales, support, QA-only, design or data entry — not a sibling engineering title.
6. Return the structured result. Keep `url` exactly as shown in the address bar or link; do not shorten or rewrite it.
7. If the site shows a login wall, CAPTCHA, "unusual activity" notice, or the page never loads: return what you have with `blocked: true` and `blockedReason`.

## Search recipes

- **LinkedIn**: `https://www.linkedin.com/jobs/search/?keywords=<query>&location=<location>&f_TPR=r<seconds>` where seconds = recencyDays * 86400 (e.g. r604800 for 7 days). Add `&f_WT=2` for remote-only. Job detail links look like `/jobs/view/<id>`.
- **Naukri**: `https://www.naukri.com/<query-with-hyphens>-jobs-in-<location>?jobAge=<days>`; job id appears in the URL. Descriptions are on the job page.
- **Indeed**: `https://in.indeed.com/jobs?q=<query>&l=<location>&fromage=<days>` (use the country domain matching the location). Detail pane loads on click; the job key `jk=` identifies a posting.
- **Wellfound**: `https://wellfound.com/jobs?q=<query>` with role filters; open each job for the description.
- **Cutshort**: open `https://cutshort.io/jobs`, search the query (and location or "remote" filter) with the page's own search, and open each job (links look like `/job/<slug>`).
- **Instahyre**: open `https://www.instahyre.com/search-jobs/` and search the query; open each opportunity for the description.
- **Hirist**: open `https://www.hirist.tech/` and search the query with the site's search; open each job for the description.
- **Foundit**: `https://www.foundit.in/srp/results?query=<query>&locations=<location>`; open each job for the description.
- **Hiring Cafe**: open `https://hiring.cafe/`, search the query with its search box and filters (location, remote, date posted); each result links to the employer's own posting: open that for the description, and use its URL as `url`.
- **Welcome to the Jungle**: `https://www.welcometothejungle.com/en/jobs?query=<query>` with the location/remote filters; open each job (`/en/companies/<company>/jobs/<slug>`).
- **Himalayas** (remote jobs): `https://himalayas.app/jobs?q=<query>`; filter by the candidate's country/time zone where offered; open each job (`/companies/<company>/jobs/<slug>`).
- **Y Combinator** (Work at a Startup, YC companies): `https://www.workatastartup.com/jobs?query=<query>` (uses the candidate's signed-in account; if it asks to sign in, return `blocked`); open each role for the description.
- **Greenhouse / Lever / Workday / company pages**: open the given careers URL, use its search, and read each posting page.

### LinkedIn Posts (hiring posts that ask for resumes by email)

`source` = "LinkedIn Posts". Recruiters and engineers post openings in the feed ("We're hiring a Node.js developer, send your resume to hr@company.com"); these never appear in LinkedIn Jobs.

1. Search posts, newest first, within the recency window: `https://www.linkedin.com/search/results/content/?keywords=<query> hiring email resume&datePosted=%22past-week%22&sortBy=%22date_posted%22` (use `past-24h` or `past-month` to match `recencyDays`). Run it for every query, and also try "<query> send CV" and "<query> share resume".
2. Keep a post only when it is a real opening for the candidate's kind of role **and** it gives an email address to send the resume to. Expand "…see more" to read the whole post; the address is often at the end or in an image caption. Skip posts that only say "DM me", "comment interested" or link to a form, and posts by candidates looking for jobs.
3. For each kept post return: `title` (the role as written), `company` (the hiring company named in the post, or the poster's current company), `url` = the post's permalink (`https://www.linkedin.com/feed/update/urn:li:activity:<id>/`, from the post's "…" menu → "Copy link to post" or the timestamp link), `sourceJobId` = the activity id, `applyEmail` = the address exactly as written, `contactName` = the poster's name, `location`, `postedAt`, `description` = the full post text, `skills`/`requirements` from the text, and `detailsComplete: true`.
4. Never comment, react, message, follow or connect.

## Output

Return only the JSON structure defined by the discovery schema. Never invent postings. `notes` may describe what was searched and how many results were seen.
