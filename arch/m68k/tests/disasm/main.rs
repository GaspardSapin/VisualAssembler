//! Tests for `src/disasm/` (m2-assembler.md §10.1, exit criterion E4), run by
//! `cargo test -p va-arch-m68k --test disasm`: golden decodes per mode and family, `Truncated`
//! and `Illegal`, and all 65,536 opwords (`decode`), the canonical text and its tokens
//! (`format`), and the flow kinds and targets (`flow`).
//!
//! This file belongs to the WP0 scaffold and stays as written (§1): WP-E owns the module files
//! and keeps its helpers there.
#![allow(clippy::unwrap_used)]

mod decode;
mod flow;
mod format;
