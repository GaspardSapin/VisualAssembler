//! The CPU architectures compiled into this binary (m1-core-isa-lexer.md §7.1, §8).
//!
//! Each CPU crate is an optional Cargo feature (`arch-m68k`, on by default). A build without any
//! `arch-*` feature has no architecture: `va info` lists none, and `va lex` and `va isa` fail with
//! `no-architecture` (exit 3). There is no `--arch` flag until a second CPU exists; M4's
//! `project.toml` will choose one.

use va_core::arch::Architecture;

/// The registered architectures, default first.
static ARCHITECTURES: &[&dyn Architecture] = &[
    #[cfg(feature = "arch-m68k")]
    &va_arch_m68k::M68K,
];

/// Returns every architecture compiled into this binary, the default one first.
pub fn architectures() -> Vec<&'static dyn Architecture> {
    ARCHITECTURES.to_vec()
}

/// Returns the architecture used by `va lex` and `va isa` (the first registered one), or `None`
/// when the binary was built without any `arch-*` feature.
pub fn default_architecture() -> Option<&'static dyn Architecture> {
    ARCHITECTURES.first().copied()
}
