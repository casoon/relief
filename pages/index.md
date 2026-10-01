---
title: About Relief
description: What Relief is, who it is for and what it deliberately is not.
order: 1
---

A normal website is a flat surface: headings, buttons, forms and dialogs
arranged for people who see the screen and use a mouse. Screen readers and
voice control make that surface usable, but they present it element by element.
They do not tell you what kind of page you are on, which parts belong together
or what the main action is.

**Relief** is a research browser that tries to close that gap. It is a fork of
Chromium that reads the page’s accessibility tree inside the browser process,
builds a semantic model of the page from it and lets people operate the page
through that model.

## Goal

People who cannot or do not want to use a mouse, or who cannot see the screen,
should be able to understand a web page and complete tasks on it reliably — by
keyboard, by speech or with their screen reader — without Relief ever acting on
a guess.

## Two lines, one core

- **Assistance.** A browser that explains the page, offers a command bar and
  speech, jump hints for the keyboard, a form assistant, consent-dialog
  handling and a simplified semantic view.
- **Testing.** The same core as a test mode for developers and auditors: form
  journeys, focus after errors, live announcements and what a real screen
  reader says, checked in the real browser.

## What Relief is not

- Not a replacement for VoiceOver, NVDA or JAWS. It runs next to them.
- Not an AI browser. A model may propose a missing name or an intent; it never
  triggers an action.
- Not a promise of WCAG conformance. Findings are findings, not a certificate.
- Not finished. Relief is a research project in active development. The
  source is public under the MIT licence, but there are no builds for others
  yet.

## Where to go next

- [Roadmap](project/roadmap/) — phases, what is done, what comes next
- [Architecture](project/architecture/) — from Chromium to the page model and back
- [Principles](project/principles/) — rules every feature has to follow
- [Testing mode](project/testing/) — accessibility tests from the user’s side
