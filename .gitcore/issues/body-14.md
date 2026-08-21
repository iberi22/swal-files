<!-- CANONICAL UNIFIED ISSUE TEMPLATE v3.8.0 -->
## 1. Executive Summary & Intent
Implement **Semantic Auto-Tagger & Natural Language Query Parser** (`feat-files-14`) for `swal-files` in `swal-files-agent`.
This task delivers an autonomous, high-performance module conforming to SWAL Files specifications (inspired by files-community/Files, macOS Finder & Yazi).

---

## 2. Target File Island (100% Disjoint)
- `crates/swal-files-agent/src/tagger.rs`
- `crates/swal-files-agent/src/lib.rs`

> [!IMPORTANT]
> Do NOT touch or edit files outside this designated file island to maintain zero merge conflicts with concurrent Jules tasks.

---

## 3. Technical Requirements & Architecture
- Crate: `swal-files-agent`
- Language: Rust (2021 Edition)
- Safety: Zero `unsafe` blocks, zero compiler warnings.
- Concurrency: Non-blocking async Tokio primitives where applicable.

---

## 4. Executable Acceptance Criteria
- [ ] Implement all structs, methods, and functions required for `Semantic Auto-Tagger & Natural Language Query Parser`.
- [ ] Export public module in `lib.rs` cleanly.
- [ ] Write comprehensive unit tests in `#[cfg(test)]` covering edge cases.
- [ ] Ensure `cargo test -p swal-files-agent` passes with 0 failures and 0 warnings.

---

## 5. Verification Commands
```bash
cargo check -p swal-files-agent
cargo test -p swal-files-agent
```
