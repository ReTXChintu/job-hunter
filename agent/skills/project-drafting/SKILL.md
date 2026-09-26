# Skill: project-drafting

Turn the candidate's own description of a project into one structured
`projects` entry for their profile. The profile is the source of truth for
every tailored resume, so accuracy matters more than polish.

## Inputs

- `description`: the candidate's words about the project: what it is, their
  role, what they built, the technologies, any results.
- `employers`: the candidate's experiences (`company`, `role`, dates), to link
  the project to one.
- `employer` (optional): the employer the candidate picked; use it.

## Rules

- Use only what the description states. Never invent features, metrics,
  numbers, team sizes, clients, users, or results. No number that isn't in
  the description.
- `name`: the project's name as the candidate gives it. If none is given,
  a short plain descriptive name (e.g. "School Management System").
- `description`: one or two sentences, in resume style, saying what the
  project is and does.
- `role`: the candidate's role as they state it (e.g. "Full Stack Developer &
  Team Lead"). If not stated, use their role at the linked employer.
- `technologies`: every technology, framework, service or API named, with
  canonical spelling ("React.js", "Node.js", "MongoDB"). Expand stack
  acronyms the candidate uses (MERN = MongoDB, Express.js, React.js, Node.js).
- `responsibilities`: what the candidate did, as action-verb resume bullets
  (Built, Designed, Integrated, Led, ...). Split the description's facts into
  separate bullets; don't pad. Leadership bullets only if they say they led.
- `achievements`: only concrete outcomes the description states (delivered,
  shipped, improved, reduced, N variants, ...). Empty when none are stated.
- `experienceCompany`: the employer's `company` exactly as listed in
  `employers`, when the description or the chosen employer makes it clear;
  otherwise empty.
- `url`: only if the description includes one.

## Output

Only the JSON structure defined by the project-draft schema.
