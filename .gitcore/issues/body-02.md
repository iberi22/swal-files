# [Ola 1.02] feat-files-02 — Async Directory Scanner & Multi-Criteria Sorter

> Ola 1 — Core Architecture, Git Engine & Preview Decoders.
> Labels: `ola1`, `wave-1`, `jules`

---

## Current State (MEDIBLE)
- Feature: `feat-files-02` at 0% in `.gitcore/features.json`
- File: `crates/swal-files-core/src/scanner.rs` (0 lines, stub)
- Tests: 0 existing, 0 passing

## Desired State (DELTA)
- **Specific Addition**: Implement non-blocking async directory traversal with Tokio, natural sorting, and hidden file filtering.
- **Primary Structs / Enums**: `DirectoryScanner`, `ScanOptions`, `SortField`, `SortOrder`
- **File Target**: `crates/swal-files-core/src/scanner.rs`

## Web Research Required
1. search: "rust tokio async directory traversal walkdir non-blocking"
2. search: "rust sort files by date size name alphanumeric natural sort"
3. search: "yazi file manager directory scanner async rust"

## Acceptance Criteria (VERIFICABLES POR COMANDO)
- [ ] `cargo check -p swal-files-core` — 0 errors, 0 warnings
- [ ] `cargo test -p swal-files-core` — all unit tests pass
- [ ] `grep -rn "DirectoryScanner" crates/swal-files-core/src/scanner.rs` >= 1 match
- [ ] Implement robust error handling (no `unwrap()` in production paths)

## Files to Modify
| File | Current State | Change | Risk |
|------|--------------|--------|------|
| `crates/swal-files-core/src/scanner.rs` | Stub (0 lines) | Complete Implementation | LOW |

## DO NOT touch
- Other crates or module files outside `crates/swal-files-core/src/scanner.rs` (assigned to concurrent tasks in Wave 1).
- `.gitcore/features.json` — reconciled at wave completion.

## Anti-Hallucination Guard
1. READ before write: inspect all existing files and `Cargo.toml` in `swal-files-core`.
2. Follow Rust 2021 idiomatic patterns with zero unsafe code.
3. Keep the file focused and concise (<180 lines of clean code).

## Merge Order
- **Merge order within wave:** 2 of 15
- **Expected effort:** Small (<30m)
- **Parallel with:** All other Wave 1 issues (100% disjoint file islands)
