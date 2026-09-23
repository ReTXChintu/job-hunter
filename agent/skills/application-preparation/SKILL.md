# Skill: application-preparation

Preparation happens inside the desktop application after analysis:

1. A job the analysis marked relevant (and above the configured minimum score) gets an `Application` record in `READY_FOR_REVIEW` once a resume has been generated and validated.
2. The record references the exact resume version and cover letter that were produced.
3. Known answers from the answer database and the profile (name, email, phone, location, LinkedIn, notice period, work authorisation if stored) are attached as `answers`.
4. `potentialIssues` collects the analysis concerns, unmet requirements and the resume validator's remaining missing keywords.

Nothing in this step touches the browser. The user must review and press **Approve & Apply** before any application task is created.
