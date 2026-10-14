//! `xtask`: the developer task runner behind `make` (bench, ci, docs, hooks).
//!
//! The binary (`src/main.rs`) only parses the command line and dispatches to the modules below;
//! everything testable lives in this library. Normative specification:
//! `docs/specs/m0-foundations.md` §6.
//!
//! Allowed internal dependencies: none (architecture rule R5). xtask drives cargo, git, Python and
//! the built binaries as child processes.

pub mod bench;
pub mod ci;
pub mod docs;
pub mod exec;
pub mod hooks;
pub mod layering;
pub mod locate;
