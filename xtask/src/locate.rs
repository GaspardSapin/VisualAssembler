//! Locations xtask works with: the workspace root, the cargo program and the target directory.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::exec;

/// Returns the workspace root: the parent of this crate's manifest directory.
///
/// Fails when that folder has no `Cargo.toml`, which means xtask was moved out of the workspace.
pub fn workspace_root() -> Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.parent().with_context(|| {
        format!(
            "the xtask manifest directory {} has no parent",
            manifest_dir.display()
        )
    })?;
    if !root.join("Cargo.toml").is_file() {
        bail!("no Cargo.toml in the workspace root {}", root.display());
    }
    Ok(root.to_path_buf())
}

/// Returns the cargo program to run: `$CARGO` (set by cargo for `cargo xtask`), else `cargo`.
pub fn cargo() -> OsString {
    std::env::var_os("CARGO")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| OsString::from("cargo"))
}

/// Runs `cargo metadata --format-version 1 --no-deps` in `root` and returns the parsed JSON.
pub fn cargo_metadata(root: &Path) -> Result<Value> {
    let mut cmd = Command::new(cargo());
    cmd.current_dir(root)
        .args(["metadata", "--format-version", "1", "--no-deps"]);
    let stdout = exec::stdout_of(&mut cmd)?;
    serde_json::from_str(&stdout).context("`cargo metadata` printed invalid JSON")
}

/// Extracts `target_directory` from `cargo metadata` output.
pub fn target_directory_from(metadata: &Value) -> Result<PathBuf> {
    metadata
        .get("target_directory")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .context("`cargo metadata` output has no string `target_directory`")
}

/// Returns the cargo target directory of the workspace at `root` (honours `CARGO_TARGET_DIR`).
pub fn target_directory(root: &Path) -> Result<PathBuf> {
    target_directory_from(&cargo_metadata(root)?)
}
