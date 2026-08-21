<!-- CANONICAL UNIFIED ISSUE TEMPLATE v3.8.0 -->
## 1. Executive Summary & Intent
Implement **Source Code Syntax Highlighter (Syntect)** (`feat-files-10`) for `swal-files` in `swal-files-preview`.
This task delivers an autonomous, high-performance module conforming to SWAL Files specifications (inspired by files-community/Files, macOS Finder & Yazi).

---

## 2. Target File Island (100% Disjoint)
- `crates/swal-files-preview/src/syntax.rs`
- `crates/swal-files-preview/Cargo.toml`

> [!IMPORTANT]
> Do NOT touch or edit files outside this designated file island to maintain zero merge conflicts with concurrent Jules tasks.

---

## 3. Technical Requirements & Architecture
- Crate: `swal-files-preview`
- Language: Rust (2021 Edition)
- Safety: Zero `unsafe` blocks, zero compiler warnings.
- Concurrency: Non-blocking async Tokio primitives where applicable.

---

## 4. Executable Acceptance Criteria
- [ ] Implement all structs, methods, and functions required for `Source Code Syntax Highlighter (Syntect)`.
- [ ] Export public module in `lib.rs` cleanly.
- [ ] Write comprehensive unit tests in `#[cfg(test)]` covering edge cases.
- [ ] Ensure `cargo test -p swal-files-preview` passes with 0 failures and 0 warnings.

---

## 5. Verification Commands
```bash
cargo check -p swal-files-preview
cargo test -p swal-files-preview
```
