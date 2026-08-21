# [Ola 1.15] feat-files-15 — Hybrid Omnibar Command & Breadcrumb Path Parser

> Ola 1 — Core Architecture, Git Engine & Preview Decoders.
> Labels: `ola1`, `wave-1`, `jules`

---

## Current State (MEDIBLE)
- Feature: `feat-files-15` at 0% in `.gitcore/features.json`
- File: `crates/swal-files-app/src/omnibar.rs` (0 lines, stub)
- Tests: 0 existing, 0 passing

## Desired State (DELTA)
- **Specific Addition**: Implement hybrid Omnibar parsing path breadcrumb pills, search mode (?), shell mode (>), and agent prompts (@).
- **Primary Structs / Enums**: `OmnibarEngine`, `BreadcrumbSegment`, `OmnibarMode`, `CommandSuggestion`
- **File Target**: `crates/swal-files-app/src/omnibar.rs`

## Web Research Required
1. search: "rust breadcrumb path parser segment clickable tokens"
2. search: "files community hybrid omnibar path search shell command palette"
3. search: "rust fuzzy search nucleo skim path matching"

## Acceptance Criteria (VERIFICABLES POR COMANDO)
- [ ] `cargo check -p swal-files-app` — 0 errors, 0 warnings
- [ ] `cargo test -p swal-files-app` — all unit tests pass
- [ ] `grep -rn "OmnibarEngine" crates/swal-files-app/src/omnibar.rs` >= 1 match
- [ ] Implement robust error handling (no `unwrap()` in production paths)

## Files to Modify
| File | Current State | Change | Risk |
|------|--------------|--------|------|
| `crates/swal-files-app/src/omnibar.rs` | Stub (0 lines) | Complete Implementation | LOW |

## DO NOT touch
- Other crates or module files outside `crates/swal-files-app/src/omnibar.rs` (assigned to concurrent tasks in Wave 1).
- `.gitcore/features.json` — reconciled at wave completion.

## Anti-Hallucination Guard
1. READ before write: inspect all existing files and `Cargo.toml` in `swal-files-app`.
2. Follow Rust 2021 idiomatic patterns with zero unsafe code.
3. Keep the file focused and concise (<180 lines of clean code).

## Merge Order
- **Merge order within wave:** 15 of 15
- **Expected effort:** Small (<30m)
- **Parallel with:** All other Wave 1 issues (100% disjoint file islands)
