# Skill: project-selection

Choose which of the candidate's projects to feature on their public job-site
profiles (LinkedIn, Naukri, ...). A profile with a few strong projects reads
better than one listing everything. The candidate reviews your choice before
anything is published.

## Inputs

- `targetRoles`, `skills`, `experiences`: what the candidate is looking for
  and where they worked.
- `projects`: every project, each with an `id`.

## How to choose

- Pick the **3 to 5** strongest projects (all of them when there are 3 or
  fewer). Fewer strong ones beat more weak ones.
- Strong means, in order of weight:
  1. relevant to the target roles and the skills recruiters will search for;
  2. substantial: real responsibilities, a clear product, stated outcomes;
  3. shows the candidate's level (leadership, architecture, integrations);
  4. recent, or at the current employer;
  5. adds something the other picks don't (avoid near-duplicates).
- Leave out thin entries (a name with no description), coursework-level work
  when professional projects exist, and near-duplicates of a stronger pick.
- Judge only from the data given. Don't rewrite or embellish any project.

## Output

One entry per project in `projects`, in the order you'd feature them (selected
first), using each `id` exactly as given:

- `selected`: true to feature it.
- `reason`: one short sentence the candidate will read, e.g. "Healthcare API
  integration work that matches Backend Developer roles." For projects left
  out, say why ("Similar to Signisure, which shows the same stack in more
  depth.").

Only the JSON structure defined by the project-selection schema.
