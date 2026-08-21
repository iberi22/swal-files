# [Ola 1.13] feat-files-13 — Xavier Cognitive Memory REST/WebSocket Bridge (:8006)

> Ola 1 — Core Architecture, Git Engine & Preview Decoders.
> Labels: `ola1`, `wave-1`, `jules`

---

## Current State (MEDIBLE)
- Feature: `feat-files-13` at 0% in `.gitcore/features.json`
- File: `crates/swal-files-agent/src/client.rs` (0 lines, stub)
- Tests: 0 existing, 0 passing

## Desired State (DELTA)
- **Specific Addition**: Implement async HTTP/WebSocket client communicating with Xavier Node Core (:8006) for file insights.
- **Primary Structs / Enums**: `XavierClient`, `AgentQueryRequest`, `AgentQueryResponse`, `MemoryIndexPayload`
- **File Target**: `crates/swal-files-agent/src/client.rs`

## Web Research Required
1. search: "rust reqwest async rest client json error handling tokio"
2. search: "rag vector database file indexing client rust"
3. search: "swal xavier memory core rest api bridge"

## Acceptance Criteria (VERIFICABLES POR COMANDO)
- [ ] `cargo check -p swal-files-agent` — 0 errors, 0 warnings
- [ ] `cargo test -p swal-files-agent` — all unit tests pass
- [ ] `grep -rn "XavierClient" crates/swal-files-agent/src/client.rs` >= 1 match
- [ ] Implement robust error handling (no `unwrap()` in production paths)

## Files to Modify
| File | Current State | Change | Risk |
|------|--------------|--------|------|
| `crates/swal-files-agent/src/client.rs` | Stub (0 lines) | Complete Implementation | LOW |

## DO NOT touch
- Other crates or module files outside `crates/swal-files-agent/src/client.rs` (assigned to concurrent tasks in Wave 1).
- `.gitcore/features.json` — reconciled at wave completion.

## Anti-Hallucination Guard
1. READ before write: inspect all existing files and `Cargo.toml` in `swal-files-agent`.
2. Follow Rust 2021 idiomatic patterns with zero unsafe code.
3. Keep the file focused and concise (<180 lines of clean code).

## Merge Order
- **Merge order within wave:** 13 of 15
- **Expected effort:** Small (<30m)
- **Parallel with:** All other Wave 1 issues (100% disjoint file islands)
