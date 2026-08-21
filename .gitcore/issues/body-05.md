# [Ola 1.05] feat-files-05 — Multi-Tab State Manager & Dual-Pane Coordinator

> Ola 1 — Core Architecture, Git Engine & Preview Decoders.
> Labels: `ola1`, `wave-1`, `jules`

---

## Current State (MEDIBLE)
- Feature: `feat-files-05` at 0% in `.gitcore/features.json`
- File: `crates/swal-files-core/src/tabs.rs` (0 lines, stub)
- Tests: 0 existing, 0 passing

## Desired State (DELTA)
- **Specific Addition**: Implement multi-tab lifecycle (create, switch, close, reorder) and split dual-pane view state.
- **Primary Structs / Enums**: `TabManager`, `TabState`, `PaneLayout`, `HistoryStack`
- **File Target**: `crates/swal-files-core/src/tabs.rs`

## Web Research Required
1. search: "rust multi tab state manager browser style active tab history"
2. search: "dual pane file manager state coordinator rust"
3. search: "files community tabs tear off drag drop architecture"

## Acceptance Criteria (VERIFICABLES POR COMANDO)
- [ ] `cargo check -p swal-files-core` — 0 errors, 0 warnings
- [ ] `cargo test -p swal-files-core` — all unit tests pass
- [ ] `grep -rn "TabManager" crates/swal-files-core/src/tabs.rs` >= 1 match
- [ ] Implement robust error handling (no `unwrap()` in production paths)

## Files to Modify
| File | Current State | Change | Risk |
|------|--------------|--------|------|
| `crates/swal-files-core/src/tabs.rs` | Stub (0 lines) | Complete Implementation | LOW |

## DO NOT touch
- Other crates or module files outside `crates/swal-files-core/src/tabs.rs` (assigned to concurrent tasks in Wave 1).
- `.gitcore/features.json` — reconciled at wave completion.

## Anti-Hallucination Guard
1. READ before write: inspect all existing files and `Cargo.toml` in `swal-files-core`.
2. Follow Rust 2021 idiomatic patterns with zero unsafe code.
3. Keep the file focused and concise (<180 lines of clean code).

## Merge Order
- **Merge order within wave:** 5 of 15
- **Expected effort:** Small (<30m)
- **Parallel with:** All other Wave 1 issues (100% disjoint file islands)
