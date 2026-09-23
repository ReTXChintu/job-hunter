# Skill: job-analysis

Compare job postings with the candidate's real profile and produce a structured relevance analysis for each.

## Inputs

- `candidate`: profile, skills, experience records, projects, preferences (target roles, locations, remote preference, minimum experience, salary, employment types).
- `jobs`: one or more postings, each with a `jobId`.

## Rules

- Use only the candidate data supplied. Do not assume skills that are not listed.
- `matchScore` (0-100) is for sorting and explanation. Be calibrated: 85+ means the candidate clearly meets the core requirements; 60-84 means a plausible fit with gaps; below 60 means significant gaps.
- `relevant` is true when the role is in the candidate's discipline, the seniority is within reach, and the location/remote arrangement is acceptable to the candidate. A missing nice-to-have does not make a job irrelevant. A hard requirement the candidate clearly cannot meet (e.g. 10+ years when they have 3, a mandatory degree they lack, on-site in a city they refuse) makes it irrelevant.
- `matchedSkills` / `missingSkills` refer to skills the posting asks for.
- `requiredExperienceMet`: compare the posting's stated years with the candidate's total experience.
- `seniorityMatch`, `locationMatch`, `employmentTypeMatch`: true / false based on the candidate preferences.
- `salaryAssessment`: short text; "not stated" when the posting has no salary.
- `concerns`: anything the user should know before applying (visa requirements, shift work, contract length, unclear employer, staffing agency, duplicate posting).
- `importantKeywords`: ATS keywords from the posting that a resume should contain if truthful.
- `summary`: two to four sentences explaining why the job does or does not fit.

## Output

Only the JSON structure defined by the analysis schema, one entry per input job, using the given `jobId` values verbatim.
