# [Ola 1.01] feat-files-01 — Core File Entry Types & Metadata Contract

> Ola 1 — Core Architecture, Git Engine & Preview Decoders.
> Labels: `ola1`, `wave-1`, `jules`

---

## Current State (MEDIBLE)
- Feature: `feat-files-01` at 0% in `.gitcore/features.json`
- File: `crates/swal-files-core/src/types.rs` (0 lines, stub)
- Tests: 0 existing, 0 passing

## Desired State (DELTA)
- **Specific Addition**: Define strongly-typed POSIX metadata, MIME categorization, and serialization for FileEntry.
- **Primary Structs / Enums**: `FileEntry`, `FileType`, `FileMetadata`, `PermissionsInfo`, `MimeCategory`
- **File Target**: `crates/swal-files-core/src/types.rs`

## Web Research Required
1. search: "rust file manager metadata permissions posix mime type"
2. search: "serde serialize file entry unix permissions chrono"
3. search: "rust std fs metadata file_type is_symlink"

## Acceptance Criteria (VERIFICABLES POR COMANDO)
- [ ] `cargo check -p swal-files-core` — 0 errors, 0 warnings
- [ ] `cargo test -p swal-files-core` — all unit tests pass
- [ ] `grep -rn "FileEntry" crates/swal-files-core/src/types.rs` >= 1 match
- [ ] Implement robust error handling (no `unwrap()` in production paths)

## Files to Modify
| File | Current State | Change | Risk |
|------|--------------|--------|------|
| `crates/swal-files-core/src/types.rs` | Stub (0 lines) | Complete Implementation | LOW |

## DO NOT touch
- Other crates or module files outside `crates/swal-files-core/src/types.rs` (assigned to concurrent tasks in Wave 1).
- `.gitcore/features.json` — reconciled at wave completion.

## Anti-Hallucination Guard
1. READ before write: inspect all existing files and `Cargo.toml` in `swal-files-core`.
2. Follow Rust 2021 idiomatic patterns with zero unsafe code.
3. Keep the file focused and concise (<180 lines of clean code).

## Merge Order
- **Merge order within wave:** 1 of 15
- **Expected effort:** Small (<30m)
- **Parallel with:** All other Wave 1 issues (100% disjoint file islands)
