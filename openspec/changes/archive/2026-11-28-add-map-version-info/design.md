# Design

## Context

The map window uses the same menu bar on the welcome screen and on an open map. Its command dispatcher handles application-level commands before checking for an open map.

## Goals / Non-Goals

**Goal**

- Show the running version from either state of the map window.

## Decisions

- Add the command to Help and dispatch it alongside the other application-level commands.
- Read Cargo's package version at compile time so the display follows the version used by the build.
- Use the existing modal message dialog for its OK button and keyboard dismissal.
