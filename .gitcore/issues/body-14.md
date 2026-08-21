# [Ola 1.14] feat-files-14 — Semantic Auto-Tagger & Natural Language Query Parser

> Ola 1 — Core Architecture, Git Engine & Preview Decoders.
> Labels: `ola1`, `wave-1`, `jules`

---

## Current State (MEDIBLE)
- Feature: `feat-files-14` at 0% in `.gitcore/features.json`
- File: `crates/swal-files-agent/src/tagger.rs` (0 lines, stub)
- Tests: 0 existing, 0 passing

## Desired State (DELTA)
- **Specific Addition**: Parse natural language queries (@find rust configs) and manage color-coded semantic tags (like macOS Finder).
- **Primary Structs / Enums**: `SemanticTagger`, `FileTag`, `ParsedAgentIntent`, `IntentKind`
- **File Target**: `crates/swal-files-agent/src/tagger.rs`

## Web Research Required
1. search: "rust natural language intent parser file search command"
2. search: "semantic file tagging color dots metadata classification"
3. search: "macOS finder tag dots file categorization rust"

## Acceptance Criteria (VERIFICABLES POR COMANDO)
- [ ] `cargo check -p swal-files-agent` — 0 errors, 0 warnings
- [ ] `cargo test -p swal-files-agent` — all unit tests pass
- [ ] `grep -rn "SemanticTagger" crates/swal-files-agent/src/tagger.rs` >= 1 match
- [ ] Implement robust error handling (no `unwrap()` in production paths)

## Files to Modify
| File | Current State | Change | Risk |
|------|--------------|--------|------|
| `crates/swal-files-agent/src/tagger.rs` | Stub (0 lines) | Complete Implementation | LOW |

## DO NOT touch
- Other crates or module files outside `crates/swal-files-agent/src/tagger.rs` (assigned to concurrent tasks in Wave 1).
- `.gitcore/features.json` — reconciled at wave completion.

## Anti-Hallucination Guard
1. READ before write: inspect all existing files and `Cargo.toml` in `swal-files-agent`.
2. Follow Rust 2021 idiomatic patterns with zero unsafe code.
3. Keep the file focused and concise (<180 lines of clean code).

## Merge Order
- **Merge order within wave:** 14 of 15
- **Expected effort:** Small (<30m)
- **Parallel with:** All other Wave 1 issues (100% disjoint file islands)
