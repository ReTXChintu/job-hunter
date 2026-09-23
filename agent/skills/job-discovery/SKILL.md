# Skill: job-discovery

Find recent job postings on one job source using Claude in Chrome.

## Inputs

- `source`: one of LinkedIn, Naukri, Indeed, Wellfound, Greenhouse, Lever, Workday, or a company careers URL.
- `queries`: search phrases derived from the candidate's target roles.
- `locations`, `remotePreference`, `recencyDays`, `maxJobs`.
- Optional `seenUrls`: postings already known; skip them.

## Procedure

1. Call `tabs_context_mcp` with `createIfEmpty: true`, then open **one** new tab with `tabs_create_mcp` and work in it.
2. Navigate to the search URL for the source (recipes below). Prefer URL parameters over typing into search boxes.
3. Read the results with `get_page_text` (or `read_page` when structure matters). Scroll or paginate only as needed to collect up to `maxJobs` postings within the recency window.
4. For each candidate posting that matches the queries, open the posting (same tab or the results detail pane) and extract the full description, requirements, responsibilities and skills. Set `detailsComplete: true` only when the full description was read.
5. Skip postings older than `recencyDays`, postings already in `seenUrls`, and obviously irrelevant roles (different discipline, wrong seniority by title such as "Director" or "Intern" when not targeted).
6. Return the structured result. Keep `url` exactly as shown in the address bar or link; do not shorten or rewrite it.
7. If the site shows a login wall, CAPTCHA, "unusual activity" notice, or the page never loads: return what you have with `blocked: true` and `blockedReason`.

## Search recipes

- **LinkedIn**: `https://www.linkedin.com/jobs/search/?keywords=<query>&location=<location>&f_TPR=r<seconds>` where seconds = recencyDays * 86400 (e.g. r604800 for 7 days). Add `&f_WT=2` for remote-only. Job detail links look like `/jobs/view/<id>`.
- **Naukri**: `https://www.naukri.com/<query-with-hyphens>-jobs-in-<location>?jobAge=<days>`; job id appears in the URL. Descriptions are on the job page.
- **Indeed**: `https://in.indeed.com/jobs?q=<query>&l=<location>&fromage=<days>` (use the country domain matching the location). Detail pane loads on click; the job key `jk=` identifies a posting.
- **Wellfound**: `https://wellfound.com/jobs?q=<query>` with role filters; open each job for the description.
- **Greenhouse / Lever / Workday / company pages**: open the given careers URL, use its search, and read each posting page.

## Output

Return only the JSON structure defined by the discovery schema. Never invent postings. `notes` may describe what was searched and how many results were seen.
