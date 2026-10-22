# Design

## Context

The unsaved changes question remembers what it was asked for (New, Open, Exit). Save saved right away when the map had a file and then went on; without a file it opened Save As, a file dialog whose result arrives later as a message. Only Exit survived that round trip, as a Save As variant of its own.

## Goals / Non-Goals

**Goals:**
- Save goes on with what was asked for, with or without Save As in between.
- One way of saving: save, then go on.

**Non-Goals:**
- Changing the question, its answers or what Cancel and Don't Save do.

## Decisions

- **Save As carries the follow-up**: the file dialog's purpose is "Save As, then optionally New / Open / Exit". Saving to a file takes the same optional follow-up and runs it only after a successful save, whether the file was known or just chosen. The separate Save-As-then-exit variant goes away.
- **Cancelling Save As** ends there, as before: no file, no save, nothing follows.

## Risks / Trade-offs

- Opening another map after Save As shows a second file dialog right after the first one; that is what the user asked for, and Cancel there keeps the saved map open.
