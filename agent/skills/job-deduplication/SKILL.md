# Skill: job-deduplication

De-duplication is performed deterministically by the desktop application (Rust) using company, title, location, description similarity, external job id and canonical URL. Claude is not asked to de-duplicate.

When discovering jobs, help the deterministic step by:

- reporting the posting's own identifier as `sourceJobId` when visible (LinkedIn `/jobs/view/<id>`, Indeed `jk=`, Naukri job id in the URL, Greenhouse/Lever ids);
- keeping the company name as written by the employer (not "Company via Recruiter");
- reporting the location as written;
- returning the full description so similarity can be measured.

If you notice the same posting twice on one results page, report it once.
