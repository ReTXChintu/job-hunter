# Skill: job-extraction

Open one job posting URL and extract its complete details.

## Inputs

- `url`, plus whatever partial data is already known (title, company, location).

## Procedure

1. Ensure a tab group exists (`tabs_context_mcp` with `createIfEmpty: true`), open a tab, navigate to `url`.
2. If the page redirects to a login wall, CAPTCHA, or an "expired/closed" notice, return `found: false` (or `blocked: true`) with the reason.
3. Read the full page text. Extract:
   - `title`, `company`, `location`, `employmentType`, `remote` (Remote / Hybrid / On-site as stated), `salary` (verbatim if shown), `seniority` (as stated), `postedAt` (as shown, e.g. "2 days ago" or a date)
   - `description`: the complete description text, cleaned of navigation noise
   - `requirements`: each stated requirement as its own string
   - `responsibilities`: each stated responsibility
   - `skills`: technologies, tools and named skills mentioned
4. Set `detailsComplete: true` only when the full description was visible.
5. Do not click Apply, Save, Follow or any action button.

## Output

Only the JSON structure defined by the extraction schema.
