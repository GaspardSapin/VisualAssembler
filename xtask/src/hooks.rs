//! `cargo xtask hooks [--force]`: install `scripts/git-hooks/commit-msg` into this repository's
//! hooks folder (m0-foundations.md §6, amended by §17.7).
//!
//! The folder comes from `git rev-parse --git-path hooks`, so `core.hooksPath` and worktrees are
//! honoured. This replaces a `cp` + `chmod` recipe that would break under cmd.exe (§7).
//!
//! Safety rules (§17.7):
//!
//! - the hooks folder must lie inside the common git folder (`git rev-parse --git-common-dir`);
//!   a `core.hooksPath` pointing elsewhere (typically a global one) is refused, even with
//!   `--force`, so the project policy is never imposed on the developer's other repositories;
//! - an existing identical hook is left as is (`already installed`);
//! - an existing different hook is never overwritten silently: without `--force` the command
//!   fails and leaves it untouched; with `--force` it is renamed to
//!   `commit-msg.bak.<YYYYMMDDTHHMMSSZ>` first.

use std::{
    fs, io,
    path::{Component, Path, PathBuf},
    process::Command,
    time::SystemTime,
};

use anyhow::{Context as _, Result, bail};

use crate::{bench::timestamp, exec};

/// The hook source, relative to the workspace root.
pub const HOOK_SOURCE: &str = "scripts/git-hooks/commit-msg";

/// The hook file name inside the hooks folder.
pub const HOOK_NAME: &str = "commit-msg";

/// Infix of a replaced hook's backup name: `commit-msg.bak.<YYYYMMDDTHHMMSSZ>`.
pub const BACKUP_INFIX: &str = ".bak.";

/// Maximum number of `-2`, `-3`, … backup suffixes tried within one second.
const MAX_BACKUP_ATTEMPTS: u32 = 1000;

/// Arguments of `cargo xtask hooks`.
#[derive(Debug, Clone, Default, clap::Args)]
pub struct HooksArgs {
    /// Replace an existing, different hook; it is first renamed to `commit-msg.bak.<time>`.
    /// Never overrides the refusal of a `core.hooksPath` outside this repository.
    #[arg(long)]
    pub force: bool,
}

/// Runs `git <args>` in `root` and returns the single path it prints, joined to `root` when
/// relative, with native separators.
fn git_path(root: &Path, args: &[&str]) -> Result<PathBuf> {
    let mut cmd = Command::new("git");
    cmd.current_dir(root).args(args);
    let stdout = exec::stdout_of(&mut cmd)?;
    let reported = stdout.trim();
    if reported.is_empty() {
        bail!("`git {}` printed nothing", args.join(" "));
    }
    // Rebuild the path from its components so separators are native on Windows.
    let relative_or_absolute: PathBuf = Path::new(reported).components().collect();
    Ok(root.join(relative_or_absolute))
}

/// Asks git for the hooks folder of the repository containing `root` (absolute path).
pub fn hooks_dir(root: &Path) -> Result<PathBuf> {
    git_path(root, &["rev-parse", "--git-path", "hooks"])
}

/// Asks git for the common git folder of the repository containing `root` (absolute path); it
/// is shared by every worktree.
pub fn common_dir(root: &Path) -> Result<PathBuf> {
    git_path(root, &["rev-parse", "--git-common-dir"])
}

/// Canonicalizes `path`, which may not exist yet: its longest existing ancestor is canonicalized
/// and the missing components are appended lexically.
fn canonicalize_allowing_missing(path: &Path) -> Result<PathBuf> {
    let mut missing = Vec::new();
    let mut existing = path;
    loop {
        match fs::canonicalize(existing) {
            Ok(mut canonical) => {
                for component in missing.iter().rev() {
                    match component {
                        Component::ParentDir => {
                            canonical.pop();
                        }
                        Component::CurDir => {}
                        other => canonical.push(other.as_os_str()),
                    }
                }
                return Ok(canonical);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let (Some(parent), Some(last)) =
                    (existing.parent(), existing.components().next_back())
                else {
                    return Err(error)
                        .with_context(|| format!("cannot resolve {}", path.display()));
                };
                missing.push(last);
                existing = parent;
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("cannot resolve {}", existing.display()));
            }
        }
    }
}

/// Fails unless the hooks folder `hooks` lies inside the common git folder `common` (§17.7).
pub fn ensure_inside_repository(hooks: &Path, common: &Path) -> Result<()> {
    let common_canonical = fs::canonicalize(common)
        .with_context(|| format!("cannot resolve the git folder {}", common.display()))?;
    let hooks_canonical = canonicalize_allowing_missing(hooks)?;
    if !hooks_canonical.starts_with(&common_canonical) {
        bail!(
            "core.hooksPath points outside this repository ({}); refusing to install a project hook there. Install it yourself or unset core.hooksPath.",
            hooks.display()
        );
    }
    Ok(())
}

/// Returns a backup path `<dir>/commit-msg.bak.<YYYYMMDDTHHMMSSZ>` for `now` that does not exist
/// yet (`-2`, `-3`, … are appended within the same second).
pub fn backup_path(dir: &Path, now: SystemTime) -> Result<PathBuf> {
    let base = format!("{HOOK_NAME}{BACKUP_INFIX}{}", timestamp::utc_compact(now));
    for attempt in 1..=MAX_BACKUP_ATTEMPTS {
        let name = if attempt == 1 {
            base.clone()
        } else {
            format!("{base}-{attempt}")
        };
        let candidate = dir.join(name);
        match fs::symlink_metadata(&candidate) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(candidate),
            Ok(_) => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("cannot inspect {}", candidate.display()));
            }
        }
    }
    bail!(
        "{MAX_BACKUP_ATTEMPTS} hook backups named {base}* already exist in {}",
        dir.display()
    );
}

/// What is currently at the hook's place.
enum Existing {
    /// Nothing.
    Absent,
    /// A regular file with the same bytes as the source.
    Identical,
    /// Anything else: other bytes, a symlink, a folder.
    Different,
}

/// Compares the current hook at `target` with `source_bytes`, never following a symlink.
fn existing_hook(target: &Path, source_bytes: &[u8]) -> Result<Existing> {
    let meta = match fs::symlink_metadata(target) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Existing::Absent),
        Err(error) => {
            return Err(error).with_context(|| format!("cannot inspect {}", target.display()));
        }
    };
    if !meta.file_type().is_file() {
        return Ok(Existing::Different);
    }
    let current = fs::read(target).with_context(|| format!("cannot read {}", target.display()))?;
    Ok(if current == source_bytes {
        Existing::Identical
    } else {
        Existing::Different
    })
}

/// Installs the hook of the workspace at `root` and returns its path (§17.7).
///
/// Prints `installed <path>` (or `already installed <path>`) on stdout. Fails, leaving every file
/// untouched, when the hooks folder is outside the common git folder (even with `force`) or when
/// a different hook exists and `force` is false.
pub fn run(root: &Path, force: bool) -> Result<PathBuf> {
    let source = root.join(HOOK_SOURCE);
    let source_bytes = fs::read(&source)
        .with_context(|| format!("cannot read the hook source {}", source.display()))?;
    let dir = hooks_dir(root)?;
    let common = common_dir(root)?;
    ensure_inside_repository(&dir, &common)?;
    fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let target = dir.join(HOOK_NAME);
    match existing_hook(&target, &source_bytes)? {
        Existing::Identical => {
            make_executable(&target)?;
            println!("already installed {}", target.display());
            return Ok(target);
        }
        Existing::Different if !force => bail!(
            "{} already exists and differs; rerun with --force to replace it (a backup is kept)",
            target.display()
        ),
        Existing::Different => {
            let backup = backup_path(&dir, SystemTime::now())?;
            fs::rename(&target, &backup).with_context(|| {
                format!(
                    "cannot move {} aside to {}",
                    target.display(),
                    backup.display()
                )
            })?;
            println!("kept the previous hook as {}", backup.display());
        }
        Existing::Absent => {}
    }
    write_new_file(&target, &source_bytes)?;
    make_executable(&target)?;
    println!("installed {}", target.display());
    Ok(target)
}

/// Creates `path` with `bytes`; fails if anything (even a dangling symlink) appeared there.
fn write_new_file(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write as _;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("cannot create {}", path.display()))?;
    file.write_all(bytes)
        .and_then(|()| file.flush())
        .with_context(|| format!("cannot write {}", path.display()))
}

/// Sets mode 0o755 so git can execute the hook.
#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .with_context(|| format!("cannot make {} executable", path.display()))
}

/// No-op on non-Unix systems: Git for Windows runs hooks through its own `sh`.
#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}
