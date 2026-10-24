//! Decoding (`src/disasm/decode.rs`; m2 §5.1, §10.1): golden decodes per mode and family,
//! `Truncated` and `Illegal`, and all 65,536 words: no panic, valid iff some form matches (WP-E).
#![allow(clippy::unwrap_used)]
