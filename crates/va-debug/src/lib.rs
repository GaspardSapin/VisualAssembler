//! Debug session: a worker thread that owns a `dyn Emulator`, command/event channels,
//! line↔address breakpoints, step over/out, run budget, undo, change tracking. No UI.
//!
//! Allowed internal dependencies: `va-core`.
//!
//! M0 skeleton: no logic yet; first content in M3.
