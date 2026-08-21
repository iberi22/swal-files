# [Ola 1.09] feat-files-09 — Git Stage Operator & Commit Builder

> Ola 1 — Core Architecture, Git Engine & Preview Decoders.
> Labels: `ola1`, `wave-1`, `jules`

---

## Current State (MEDIBLE)
- Feature: `feat-files-09` at 0% in `.gitcore/features.json`
- File: `crates/swal-files-git/src/commit.rs` (0 lines, stub)
- Tests: 0 existing, 0 passing

## Desired State (DELTA)
- **Specific Addition**: Execute non-blocking async git stage, unstage, and commit operations with author and message verification.
- **Primary Structs / Enums**: `GitOperator`, `CommitOptions`, `CommitResult`
- **File Target**: `crates/swal-files-git/src/commit.rs`

## Web Research Required
1. search: "rust git stage add unstage restore commit message author"
2. search: "rust tokio process async git command execution"
3. search: "git commit builder pre-commit check rust"

## Acceptance Criteria (VERIFICABLES POR COMANDO)
- [ ] `cargo check -p swal-files-git` — 0 errors, 0 warnings
- [ ] `cargo test -p swal-files-git` — all unit tests pass
- [ ] `grep -rn "GitOperator" crates/swal-files-git/src/commit.rs` >= 1 match
- [ ] Implement robust error handling (no `unwrap()` in production paths)

## Files to Modify
| File | Current State | Change | Risk |
|------|--------------|--------|------|
| `crates/swal-files-git/src/commit.rs` | Stub (0 lines) | Complete Implementation | LOW |

## DO NOT touch
- Other crates or module files outside `crates/swal-files-git/src/commit.rs` (assigned to concurrent tasks in Wave 1).
- `.gitcore/features.json` — reconciled at wave completion.

## Anti-Hallucination Guard
1. READ before write: inspect all existing files and `Cargo.toml` in `swal-files-git`.
2. Follow Rust 2021 idiomatic patterns with zero unsafe code.
3. Keep the file focused and concise (<180 lines of clean code).

## Merge Order
- **Merge order within wave:** 9 of 15
- **Expected effort:** Small (<30m)
- **Parallel with:** All other Wave 1 issues (100% disjoint file islands)
