---
title: Testing mode
description: Accessibility tests from the side of keyboard and screen-reader users.
order: 4
---

Static checkers look at the state of a page. Relief’s testing mode looks at the
**journey**: what happens when someone fills in a form with the keyboard, submits
it with an error, or listens to it with a screen reader.

## Tests in plain language

Test files describe steps and expectations by role and name, the way assistive
technology sees the page:

```text
do: fill Name with Erika Muster
do: !click Send message
expect: focus now on textbox "Email"
expect: invalid
```

## What it checks

- every form field has an accessible name;
- error messages are associated with their field and visible after submit;
- focus moves to the first error;
- confirmations are announced as status messages;
- an expected tab sequence reaches every field;
- what VoiceOver actually says, compared by parts rather than exact wording.

## Where it fits

For plain functional tests Playwright remains the better tool — and Relief is
planned to plug into it. The testing mode is for the part other tools miss:
the experience of people using a keyboard or a screen reader.
