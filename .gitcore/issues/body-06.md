# [Ola 1.06] feat-files-06 — Git Repository Detector & Branch Resolver

> Ola 1 — Core Architecture, Git Engine & Preview Decoders.
> Labels: `ola1`, `wave-1`, `jules`

---

## Current State (MEDIBLE)
- Feature: `feat-files-06` at 0% in `.gitcore/features.json`
- File: `crates/swal-files-git/src/detector.rs` (0 lines, stub)
- Tests: 0 existing, 0 passing

## Desired State (DELTA)
- **Specific Addition**: Implement fast upwards search for .git root directory and parse current HEAD branch without C dependencies.
- **Primary Structs / Enums**: `GitRepoDetector`, `RepoInfo`, `BranchInfo`
- **File Target**: `crates/swal-files-git/src/detector.rs`

## Web Research Required
1. search: "rust find git repository root directory upwards"
2. search: "rust parse git HEAD ref branch name without libgit2"
3. search: "rust git rev-parse show-toplevel current branch"

## Acceptance Criteria (VERIFICABLES POR COMANDO)
- [ ] `cargo check -p swal-files-git` — 0 errors, 0 warnings
- [ ] `cargo test -p swal-files-git` — all unit tests pass
- [ ] `grep -rn "GitRepoDetector" crates/swal-files-git/src/detector.rs` >= 1 match
- [ ] Implement robust error handling (no `unwrap()` in production paths)

## Files to Modify
| File | Current State | Change | Risk |
|------|--------------|--------|------|
| `crates/swal-files-git/src/detector.rs` | Stub (0 lines) | Complete Implementation | LOW |

## DO NOT touch
- Other crates or module files outside `crates/swal-files-git/src/detector.rs` (assigned to concurrent tasks in Wave 1).
- `.gitcore/features.json` — reconciled at wave completion.

## Anti-Hallucination Guard
1. READ before write: inspect all existing files and `Cargo.toml` in `swal-files-git`.
2. Follow Rust 2021 idiomatic patterns with zero unsafe code.
3. Keep the file focused and concise (<180 lines of clean code).

## Merge Order
- **Merge order within wave:** 6 of 15
- **Expected effort:** Small (<30m)
- **Parallel with:** All other Wave 1 issues (100% disjoint file islands)
