# [Ola 1.12] feat-files-12 — Image & SVG Thumbnail Generator and Dimensions Reader

> Ola 1 — Core Architecture, Git Engine & Preview Decoders.
> Labels: `ola1`, `wave-1`, `jules`

---

## Current State (MEDIBLE)
- Feature: `feat-files-12` at 0% in `.gitcore/features.json`
- File: `crates/swal-files-preview/src/image.rs` (0 lines, stub)
- Tests: 0 existing, 0 passing

## Desired State (DELTA)
- **Specific Addition**: Read image dimensions, EXIF data, and produce scaled RGB thumbnail buffers for QuickLook and grid views.
- **Primary Structs / Enums**: `ImagePreviewer`, `ImageMetadata`, `ThumbnailConfig`
- **File Target**: `crates/swal-files-preview/src/image.rs`

## Web Research Required
1. search: "rust image-rs fast thumbnail generation dimensions aspect ratio"
2. search: "rust resvg svg thumbnail rasterization wgpu"
3. search: "macos quicklook image preview exif metadata rust"

## Acceptance Criteria (VERIFICABLES POR COMANDO)
- [ ] `cargo check -p swal-files-preview` — 0 errors, 0 warnings
- [ ] `cargo test -p swal-files-preview` — all unit tests pass
- [ ] `grep -rn "ImagePreviewer" crates/swal-files-preview/src/image.rs` >= 1 match
- [ ] Implement robust error handling (no `unwrap()` in production paths)

## Files to Modify
| File | Current State | Change | Risk |
|------|--------------|--------|------|
| `crates/swal-files-preview/src/image.rs` | Stub (0 lines) | Complete Implementation | LOW |

## DO NOT touch
- Other crates or module files outside `crates/swal-files-preview/src/image.rs` (assigned to concurrent tasks in Wave 1).
- `.gitcore/features.json` — reconciled at wave completion.

## Anti-Hallucination Guard
1. READ before write: inspect all existing files and `Cargo.toml` in `swal-files-preview`.
2. Follow Rust 2021 idiomatic patterns with zero unsafe code.
3. Keep the file focused and concise (<180 lines of clean code).

## Merge Order
- **Merge order within wave:** 12 of 15
- **Expected effort:** Small (<30m)
- **Parallel with:** All other Wave 1 issues (100% disjoint file islands)
