# [Ola 1.08] feat-files-08 — Unified & Side-by-Side Git Diff Parser

> Ola 1 — Core Architecture, Git Engine & Preview Decoders.
> Labels: `ola1`, `wave-1`, `jules`

---

## Current State (MEDIBLE)
- Feature: `feat-files-08` at 0% in `.gitcore/features.json`
- File: `crates/swal-files-git/src/diff.rs` (0 lines, stub)
- Tests: 0 existing, 0 passing

## Desired State (DELTA)
- **Specific Addition**: Parse unified git diffs into structured line-by-line hunks supporting additions, deletions, and line numbers.
- **Primary Structs / Enums**: `DiffParser`, `DiffFileHunk`, `DiffLine`, `DiffLineKind`
- **File Target**: `crates/swal-files-git/src/diff.rs`

## Web Research Required
1. search: "rust parse git diff unified side by side line numbers"
2. search: "rust similar diff hunk additions deletions line parser"
3. search: "git diff side by side split view data structure rust"

## Acceptance Criteria (VERIFICABLES POR COMANDO)
- [ ] `cargo check -p swal-files-git` — 0 errors, 0 warnings
- [ ] `cargo test -p swal-files-git` — all unit tests pass
- [ ] `grep -rn "DiffParser" crates/swal-files-git/src/diff.rs` >= 1 match
- [ ] Implement robust error handling (no `unwrap()` in production paths)

## Files to Modify
| File | Current State | Change | Risk |
|------|--------------|--------|------|
| `crates/swal-files-git/src/diff.rs` | Stub (0 lines) | Complete Implementation | LOW |

## DO NOT touch
- Other crates or module files outside `crates/swal-files-git/src/diff.rs` (assigned to concurrent tasks in Wave 1).
- `.gitcore/features.json` — reconciled at wave completion.

## Anti-Hallucination Guard
1. READ before write: inspect all existing files and `Cargo.toml` in `swal-files-git`.
2. Follow Rust 2021 idiomatic patterns with zero unsafe code.
3. Keep the file focused and concise (<180 lines of clean code).

## Merge Order
- **Merge order within wave:** 8 of 15
- **Expected effort:** Small (<30m)
- **Parallel with:** All other Wave 1 issues (100% disjoint file islands)
