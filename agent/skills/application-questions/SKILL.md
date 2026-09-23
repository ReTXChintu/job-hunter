# Skill: application-questions

Rules for answering questions on application forms.

## Sources of answers, in order

1. `answers` provided by the desktop app (the reusable answer database, curated by the user).
2. Candidate profile facts (name, email, phone, location, LinkedIn, GitHub, portfolio, current title, notice period, expected salary range if the profile has one).
3. Nothing else.

## Matching

- Match questions by meaning, not exact wording ("Are you authorised to work in India?" matches "Do you have the legal right to work in India?").
- For choice fields, pick the option that exactly corresponds to the known answer. If no option corresponds, treat the question as unknown.

## Unknown questions

If a question cannot be answered from the sources above, it is unknown. Report it with:

- `question`: the exact label text
- `fieldType`: text, textarea, select, radio, checkbox, number, date, file or unknown
- `options`: the visible options for choice fields
- `required`: whether the form marks it as required
- `context`: any helper text shown near the field

Then stop before submission and return `HUMAN_INPUT_REQUIRED`.

## Never guess

Work authorisation, visa, salary, notice period, availability dates, disability, veteran status, gender, ethnicity, criminal history, references, previous employment with the company, relatives at the company, and any free-text "why do you want to work here" question that the user has not pre-answered.
