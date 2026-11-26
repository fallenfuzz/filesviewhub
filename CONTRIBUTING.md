# Contributing to FilesViewHub

The full reference is [`AGENTS.md`](AGENTS.md). It is written for AI coding agents, and every rule
in it applies to people too. This file is the short path in: setup, the rules pull requests most
often bounce on, and what review looks like.

## Setup

You need Node 24, Rust stable (rustup) and the Tauri 2 prerequisites for your system: on Windows
the Visual Studio Build Tools with the C++ workload; on Linux the packages listed under
Environment in `AGENTS.md`.

```
npm ci
npm run fetch-tools    # FFmpeg, PDFium and ddjvu, pinned by hash, into tools/
npm run tauri dev      # the dev app
node scripts/check.mjs # the gate: UI tests, type check, Rust tests, clippy
```

On macOS, skip `npm run fetch-tools`: the tools are not fetched there yet, and the gate runs
without them. From WSL, `node scripts/check.mjs` runs the gate in WSL, `--windows` runs it on Windows
through `cmd.exe` (the default when Windows npm installed `node_modules`), and `--linux` runs it in
a Linux build folder (`scripts/linux.sh`).

## Before opening a pull request

The [pull request template](.github/pull_request_template.md) is the checklist. The rules behind
its items:

- **The gate passes.** `node scripts/check.mjs` on your system. CI runs it on Windows, macOS and
  Linux.
- **A change-log entry, dated the day you write it.** One file per change under
  `change-logs/YYYY/MM/DD/<type>-<slug>.md`; see [`change-logs/README.md`](change-logs/README.md).
- **Every visible word is in `src/lib/strings.ts`.** `npm test` fails on a word written in a
  component.
- **No `cargo fmt`.** The Rust code is not rustfmt-formatted; a formatted file is a diff nobody can
  review. Clippy stays at 0 warnings (`cargo clippy --all-targets -- -D warnings`).
- **Decision records stay true.** A non-obvious choice goes in `decisions/YYYY/MM/DD/<slug>.md`.
  If your change makes an existing record untrue, add a supersede note at its top and write the new
  record in the same commit ([`decisions/README.md`](decisions/README.md)).
- **A visible change has screenshots.** Before and after, in the pull request.
- **Works on every system.** Windows, macOS and Linux; a script handed to the person ships for
  each.
- **The app never writes into a folder the person checked**, and every store, tool and network
  call shows on its "What this app does" screen.
- **English only** in code, comments, commits, entries and records.

## What review looks like

The reviewer runs the branch and checks what it claims. Say in the pull request what you checked
and on which system, and what you did not. Pull requests are squash-merged onto `main`.
