//! Tests for `src/hooks.rs` (`cargo xtask hooks [--force]`, m0-foundations.md §17.7) in temporary
//! git repositories under `CARGO_TARGET_TMPDIR` (§17.9; `git` is required, as in CI), and for the
//! behaviour of `scripts/git-hooks/commit-msg` (§17.6), run with `sh`.
#![allow(clippy::unwrap_used)]

use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU32, Ordering},
};

use xtask::hooks::{self, HOOK_NAME, HOOK_SOURCE};

/// The hook source installed by the temporary repositories.
const NEW_HOOK: &str = "#!/bin/sh\n# project hook, new version\nexit 0\n";

/// A different hook already present in the hooks folder.
const LOCAL_HOOK: &str = "#!/bin/sh\n# the developer's own hook\nexit 0\n";

/// Returns a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` (§17.9): created with
/// `create_dir`, which fails if the path already exists.
fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    dir
}

/// Creates `base/relative` one level at a time below the fresh folder `base`.
fn make_folders(base: &Path, relative: &Path) {
    let mut path = base.to_path_buf();
    for component in relative.components() {
        path.push(component);
        match std::fs::create_dir(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => panic!("cannot create {}: {error}", path.display()),
        }
    }
}

/// Runs `git <args>` in `dir` and asserts that it succeeds.
fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Creates a fresh git repository holding [`NEW_HOOK`] at `scripts/git-hooks/commit-msg`.
///
/// Asserts that git sees it as its own repository: the temporary folder lies inside the
/// project's working tree, and a test must never fall through to the project's `.git`.
fn repo(tag: &str) -> PathBuf {
    let root = temp_dir(tag);
    git(&root, &["init", "--quiet"]);
    let common = std::fs::canonicalize(hooks::common_dir(&root).unwrap()).unwrap();
    assert!(
        common.starts_with(std::fs::canonicalize(&root).unwrap()),
        "{} is not its own git repository (git folder {})",
        root.display(),
        common.display()
    );
    let source = Path::new(HOOK_SOURCE);
    make_folders(&root, source.parent().unwrap());
    std::fs::write(root.join(source), NEW_HOOK).unwrap();
    root
}

/// The hooks folder of a repository created by [`repo`] (no `core.hooksPath`).
fn hooks_folder(root: &Path) -> PathBuf {
    root.join(".git").join("hooks")
}

/// Returns the names of the hook backups in `dir`.
fn backups(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("commit-msg.bak."))
        .collect();
    names.sort();
    names
}

/// Asserts that `left` and `right` name the same file.
fn assert_same_file(left: &Path, right: &Path) {
    assert_eq!(
        std::fs::canonicalize(left).unwrap(),
        std::fs::canonicalize(right).unwrap()
    );
}

#[test]
fn fresh_install_copies_the_hook() {
    let root = repo("hooks-fresh");
    let installed = hooks::run(&root, false).unwrap();
    let expected = hooks_folder(&root).join(HOOK_NAME);
    assert_same_file(&installed, &expected);
    assert_eq!(std::fs::read_to_string(&expected).unwrap(), NEW_HOOK);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(&expected).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
    }
    assert_eq!(backups(&hooks_folder(&root)), Vec::<String>::new());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn rerun_with_identical_content_is_idempotent() {
    let root = repo("hooks-rerun");
    let first = hooks::run(&root, false).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&first, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
    let second = hooks::run(&root, false).unwrap();
    let forced = hooks::run(&root, true).unwrap();
    assert_eq!(first, second);
    assert_eq!(first, forced);
    assert_eq!(std::fs::read_to_string(&first).unwrap(), NEW_HOOK);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(&first).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755, "the Unix mode is re-applied");
    }
    assert_eq!(backups(&hooks_folder(&root)), Vec::<String>::new());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn differing_hook_without_force_is_refused_and_left_unchanged() {
    let root = repo("hooks-differs");
    let target = hooks_folder(&root).join(HOOK_NAME);
    make_folders(&root, Path::new(".git/hooks"));
    std::fs::write(&target, LOCAL_HOOK).unwrap();
    let error = format!("{:#}", hooks::run(&root, false).unwrap_err());
    assert!(
        error.contains(
            "already exists and differs; rerun with --force to replace it (a backup is kept)"
        ),
        "{error}"
    );
    assert_eq!(std::fs::read_to_string(&target).unwrap(), LOCAL_HOOK);
    assert_eq!(backups(&hooks_folder(&root)), Vec::<String>::new());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn force_keeps_a_timestamped_backup_and_installs_the_new_hook() {
    let root = repo("hooks-force");
    let dir = hooks_folder(&root);
    make_folders(&root, Path::new(".git/hooks"));
    let target = dir.join(HOOK_NAME);
    std::fs::write(&target, LOCAL_HOOK).unwrap();
    let installed = hooks::run(&root, true).unwrap();
    assert_same_file(&installed, &target);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), NEW_HOOK);
    let names = backups(&dir);
    assert_eq!(names.len(), 1, "{names:?}");
    let stamp = names[0].strip_prefix("commit-msg.bak.").unwrap();
    assert_eq!(stamp.len(), "YYYYMMDDTHHMMSSZ".len(), "{stamp}");
    assert!(
        stamp.chars().enumerate().all(|(index, c)| match index {
            8 => c == 'T',
            15 => c == 'Z',
            _ => c.is_ascii_digit(),
        }),
        "{stamp}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join(&names[0])).unwrap(),
        LOCAL_HOOK
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn backups_in_the_same_second_never_overwrite_each_other() {
    let dir = temp_dir("hooks-backup-names");
    let now = std::time::SystemTime::now();
    let first = hooks::backup_path(&dir, now).unwrap();
    std::fs::write(&first, "one").unwrap();
    let second = hooks::backup_path(&dir, now).unwrap();
    assert_ne!(first, second);
    assert_eq!(
        second.file_name().unwrap().to_string_lossy(),
        format!("{}-2", first.file_name().unwrap().to_string_lossy())
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `git -c core.hooksPath=<outside>` is reproduced with the repository's own configuration, the
/// only way to pass it to the in-process `hooks::run`; git resolves both the same way.
#[test]
fn hooks_path_outside_the_repository_is_refused_even_with_force() {
    let root = repo("hooks-outside");
    let outside = temp_dir("hooks-outside-folder");
    git(
        &root,
        &["config", "core.hooksPath", &outside.to_string_lossy()],
    );
    for force in [false, true] {
        let error = format!("{:#}", hooks::run(&root, force).unwrap_err());
        assert!(
            error.contains("core.hooksPath points outside this repository ("),
            "force = {force}: {error}"
        );
        assert!(
            error.contains(
                "refusing to install a project hook there. Install it yourself or unset core.hooksPath."
            ),
            "{error}"
        );
    }
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
    std::fs::remove_dir_all(&root).unwrap();
    std::fs::remove_dir_all(&outside).unwrap();
}

#[test]
fn missing_hooks_path_outside_the_repository_is_not_created() {
    let root = repo("hooks-outside-missing");
    let outside = temp_dir("hooks-outside-parent");
    let missing = outside.join("not-yet").join("hooks");
    git(
        &root,
        &["config", "core.hooksPath", &missing.to_string_lossy()],
    );
    let error = format!("{:#}", hooks::run(&root, true).unwrap_err());
    assert!(error.contains("points outside this repository"), "{error}");
    assert!(!outside.join("not-yet").exists());
    std::fs::remove_dir_all(&root).unwrap();
    std::fs::remove_dir_all(&outside).unwrap();
}

#[test]
fn hooks_path_inside_the_git_folder_is_accepted() {
    let root = repo("hooks-inside");
    git(&root, &["config", "core.hooksPath", ".git/custom-hooks"]);
    let installed = hooks::run(&root, false).unwrap();
    assert_same_file(&installed, &root.join(".git/custom-hooks").join(HOOK_NAME));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn missing_hook_source_is_an_error() {
    let root = temp_dir("hooks-no-source");
    git(&root, &["init", "--quiet"]);
    let error = format!("{:#}", hooks::run(&root, false).unwrap_err());
    assert!(error.contains("commit-msg"), "{error}");
    std::fs::remove_dir_all(&root).unwrap();
}

// ---------------------------------------------------------------------------------------------
// Behaviour of the real hook `scripts/git-hooks/commit-msg` (§17.6, §17.7).
// ---------------------------------------------------------------------------------------------

/// Runs `sh <workspace>/scripts/git-hooks/commit-msg <message_file>` and returns its exit code.
/// Returns `None` after printing `skipped: sh not found` when Windows has no `sh` on `PATH`; on
/// Unix a missing `sh` is a failure.
fn run_hook(message_file: &Path) -> Option<i32> {
    let script = xtask::locate::workspace_root().unwrap().join(HOOK_SOURCE);
    match Command::new("sh").arg(&script).arg(message_file).output() {
        Ok(output) => Some(
            output
                .status
                .code()
                .unwrap_or_else(|| panic!("the hook was killed: {output:?}")),
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound && cfg!(windows) => {
            println!("skipped: sh not found");
            None
        }
        Err(error) => panic!("cannot run sh: {error}"),
    }
}

/// Writes `message` to a fresh file and returns the hook's exit code for it.
fn hook_exit(message: &[u8]) -> Option<i32> {
    let dir = temp_dir("hook-message");
    let file = dir.join("COMMIT_EDITMSG");
    std::fs::write(&file, message).unwrap();
    let code = run_hook(&file);
    std::fs::remove_dir_all(&dir).unwrap();
    code
}

#[test]
fn hook_rejects_ai_credits() {
    for message in [
        "Fix the parser\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n".as_bytes(),
        b"Fix the parser\n\nCo-authored-by: Codex <codex@openai.com>\n",
        b"Fix the parser\n\nAssisted-by: Gemini\n",
        "Fix the parser \u{1F916}\n".as_bytes(),
        b"Fix the parser\n\nGenerated with [Claude Code](https://claude.com/claude-code)\n",
    ] {
        let Some(code) = hook_exit(message) else {
            return;
        };
        assert_eq!(code, 1, "{}", String::from_utf8_lossy(message));
    }
}

#[test]
fn hook_accepts_clean_messages() {
    for message in [
        "Fix the parser\n\nThe lexer now keeps CRLF line numbers.\n",
        "Document the literal text xf0x9fxa4x96 in the hook tests\n",
        "Fix the parser\n\n# Co-authored-by: Claude\n",
        "Fix the parser\n\n# ------------------------ >8 ------------------------\nCo-Authored-By: Claude <noreply@anthropic.com>\n",
    ] {
        let Some(code) = hook_exit(message.as_bytes()) else {
            return;
        };
        assert_eq!(code, 0, "{message}");
    }
}

#[test]
fn hook_rejects_a_missing_message_file() {
    let dir = temp_dir("hook-missing");
    if let Some(code) = run_hook(&dir.join("does-not-exist")) {
        assert_eq!(code, 1);
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

// ---------------------------------------------------------------------------------------------
// QA audit round 1 (continuation): the hook as git really runs it (E6). The tests above inherit
// the test process's locale, which is usually unset, i.e. `C`. Git for Windows starts every hook
// with `LC_CTYPE=C.UTF-8` when no locale variable is set, and developer shells usually export a
// UTF-8 `LANG`, so the hook must give the same verdicts under UTF-8 locales.
// ---------------------------------------------------------------------------------------------

/// The AI credits that §17.7 requires the hook to reject, one message each.
const AI_CREDITS: [&str; 5] = [
    "Fix the parser\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n",
    "Fix the parser\n\nCo-authored-by: Codex <codex@openai.com>\n",
    "Fix the parser\n\nAssisted-by: Gemini\n",
    "Fix the parser \u{1F916}\n",
    "Fix the parser\n\nGenerated with [Claude Code](https://claude.com/claude-code)\n",
];

/// The locale variables a hook can see.
const LOCALE_VARIABLES: [&str; 3] = ["LC_ALL", "LC_CTYPE", "LANG"];

/// Runs `sh <workspace>/scripts/git-hooks/commit-msg <file>` on `message` with exactly the locale
/// variables `locale` (every other locale variable removed). `None` after printing
/// `skipped: sh not found` when Windows has no `sh` on `PATH`, as [`run_hook`] does.
fn hook_exit_with_locale(message: &[u8], locale: &[(&str, &str)]) -> Option<i32> {
    let dir = temp_dir("hook-locale");
    let file = dir.join("COMMIT_EDITMSG");
    std::fs::write(&file, message).unwrap();
    let script = xtask::locate::workspace_root().unwrap().join(HOOK_SOURCE);
    let mut command = Command::new("sh");
    command.arg(&script).arg(&file);
    for name in LOCALE_VARIABLES {
        command.env_remove(name);
    }
    command.envs(locale.iter().copied());
    let code = match command.output() {
        Ok(output) => Some(
            output
                .status
                .code()
                .unwrap_or_else(|| panic!("the hook was killed: {output:?}")),
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound && cfg!(windows) => {
            println!("skipped: sh not found");
            None
        }
        Err(error) => panic!("cannot run sh: {error}"),
    };
    std::fs::remove_dir_all(&dir).unwrap();
    code
}

/// The robot emoji is a 4-byte UTF-8 character. Git for Windows' `grep -F` (GNU grep 3.0, MSYS2)
/// does not match such a character under a UTF-8 locale, so `grep -qF -e "$robot"` misses it
/// there, while `C` and Linux grep match it.
#[test]
fn hook_rejects_ai_credits_under_utf8_locales() {
    for locale in [
        // What Git for Windows sets for every hook when no locale variable is set.
        &[("LC_CTYPE", "C.UTF-8")][..],
        &[("LC_ALL", "C.UTF-8")][..],
        // A typical developer shell.
        &[("LANG", "en_US.UTF-8")][..],
        // QA (final M0 pass): the two remaining locales that §18.2 requires this test to cover.
        &[("LC_ALL", "en_US.UTF-8")][..],
        &[][..],
    ] {
        for message in AI_CREDITS {
            let Some(code) = hook_exit_with_locale(message.as_bytes(), locale) else {
                return;
            };
            assert_eq!(code, 1, "locale {locale:?}, message: {message}");
        }
    }
}

#[test]
fn hook_accepts_clean_messages_under_utf8_locales() {
    for locale in [
        &[("LC_CTYPE", "C.UTF-8")][..],
        &[("LANG", "en_US.UTF-8")][..],
    ] {
        for message in [
            "Fix the parser\n\nCo-authored-by: Ada Lovelace <ada@example.org>\n",
            "Translate the menu: caf\u{e9}, \u{65e5}\u{672c}\u{8a9e}, \u{2713}\n",
        ] {
            let Some(code) = hook_exit_with_locale(message.as_bytes(), locale) else {
                return;
            };
            assert_eq!(code, 0, "locale {locale:?}, message: {message}");
        }
    }
}

/// Creates a fresh repository whose `scripts/git-hooks/commit-msg` is a copy of the project's
/// real hook, installs it with `hooks::run` (what `make hooks` runs) and returns its root.
fn repo_with_project_hook(tag: &str) -> PathBuf {
    let root = temp_dir(tag);
    git(&root, &["init", "--quiet"]);
    let common = std::fs::canonicalize(hooks::common_dir(&root).unwrap()).unwrap();
    assert!(
        common.starts_with(std::fs::canonicalize(&root).unwrap()),
        "{} is not its own git repository (git folder {})",
        root.display(),
        common.display()
    );
    let source = Path::new(HOOK_SOURCE);
    make_folders(&root, source.parent().unwrap());
    let project_hook = xtask::locate::workspace_root().unwrap().join(HOOK_SOURCE);
    std::fs::copy(&project_hook, root.join(source)).unwrap();
    hooks::run(&root, false).unwrap();
    root
}

/// Asks git itself to run the installed commit-msg hook on `message` (`git hook run`, git 2.36
/// or later): the hook gets git's own shell and environment, as during `git commit`, and no
/// commit is created. The locale variables are removed so git applies its own default.
fn git_hook_exit(root: &Path, message: &[u8]) -> i32 {
    let file = root.join(".git").join("QA_COMMIT_EDITMSG");
    std::fs::write(&file, message).unwrap();
    let mut command = Command::new("git");
    command
        .current_dir(root)
        .args(["hook", "run", "commit-msg", "--"])
        .arg(&file);
    for name in LOCALE_VARIABLES {
        command.env_remove(name);
    }
    let output = command.output().unwrap();
    output
        .status
        .code()
        .unwrap_or_else(|| panic!("git hook run was killed: {output:?}"))
}

/// E6, automated without creating a commit: the hook installed by `hooks::run` and started by
/// git rejects every AI credit of §17.7 and accepts a human co-author.
#[test]
fn hook_installed_by_xtask_and_run_by_git_rejects_every_ai_credit() {
    let root = repo_with_project_hook("hook-git-run");
    for message in AI_CREDITS {
        assert_eq!(git_hook_exit(&root, message.as_bytes()), 1, "{message}");
    }
    assert_eq!(
        git_hook_exit(
            &root,
            b"Fix the parser\n\nCo-authored-by: Ada Lovelace <ada@example.org>\n"
        ),
        0
    );
    std::fs::remove_dir_all(&root).unwrap();
}

// ---------------------------------------------------------------------------------------------
// QA, final M0 pass (after §18): the trailer forms AI tools and editors really produce, and a
// real `git commit` (row 18-1: "a real commit with the robot emoji is rejected").
// ---------------------------------------------------------------------------------------------

/// The locales of §18.2 plus the one Git for Windows sets for hooks.
const HOOK_LOCALES: [&[(&str, &str)]; 5] = [
    &[],
    &[("LC_CTYPE", "C.UTF-8")],
    &[("LC_ALL", "C.UTF-8")],
    &[("LC_ALL", "en_US.UTF-8")],
    &[("LANG", "en_US.UTF-8")],
];

/// Variants of the §17.7 credits that tools or editors write: CRLF line endings (Windows
/// editors), no space or a tab after the colon, upper case, the attribution block Claude Code
/// writes, and the trailers of Copilot, Cursor and aider.
#[test]
fn hook_rejects_ai_trailers_in_the_forms_tools_and_editors_write() {
    let robot = "\u{1F916}";
    let messages = [
        "Fix the parser\r\n\r\nCo-Authored-By: Claude <noreply@anthropic.com>\r\n".to_owned(),
        "Fix the parser\n\nCo-authored-by:Claude <noreply@anthropic.com>\n".to_owned(),
        "Fix the parser\n\nCo-authored-by:\tClaude\n".to_owned(),
        "FIX THE PARSER\n\nCO-AUTHORED-BY: CLAUDE <NOREPLY@ANTHROPIC.COM>\n".to_owned(),
        format!(
            "Fix the parser\n\n{robot} Generated with [Claude Code](https://claude.com/claude-code)\n\nCo-Authored-By: Claude Opus <noreply@anthropic.com>\n"
        ),
        format!("Fix the parser\r\n\r\n{robot} Generated with [Claude Code]\r\n"),
        "Fix the parser\n\nCo-authored-by: Copilot <175728472+Copilot@users.noreply.github.com>\n"
            .to_owned(),
        "Fix the parser\n\nCo-authored-by: Cursor Agent <cursoragent@cursor.com>\n".to_owned(),
        "Fix the parser\n\nCo-authored-by: aider (gpt-4o) <noreply@aider.chat>\n".to_owned(),
        "Fix the parser\n\nGenerated-by: OpenAI Codex\n".to_owned(),
    ];
    for locale in HOOK_LOCALES {
        for message in &messages {
            let Some(code) = hook_exit_with_locale(message.as_bytes(), locale) else {
                return;
            };
            assert_eq!(code, 1, "locale {locale:?}, message: {message:?}");
        }
    }
}

/// No false positive on CRLF messages, human trailers or non-ASCII text, under every locale.
#[test]
fn hook_accepts_clean_crlf_and_non_ascii_messages_under_every_locale() {
    let messages = [
        "Fix the parser\r\n\r\nThe lexer now keeps CRLF line numbers.\r\n",
        "Fix the parser\n\nReviewed-by: Ada Lovelace <ada@example.org>\nSigned-off-by: Grace Hopper <grace@example.org>\n",
        "Fix the parser\n\nGenerated with care from the 68000 manual tables\n",
        "Corrige l'analyseur : \u{e9}\u{e8}\u{e0}, \u{65e5}\u{672c}\u{8a9e}, \u{2713} \u{1F600}\n",
    ];
    for locale in HOOK_LOCALES {
        for message in messages {
            let Some(code) = hook_exit_with_locale(message.as_bytes(), locale) else {
                return;
            };
            assert_eq!(code, 0, "locale {locale:?}, message: {message:?}");
        }
    }
}

/// Runs `git commit -F <file>` in `root` with a fixed test identity and the locale variables
/// removed, and returns the exit code.
fn git_commit_exit(root: &Path, message: &[u8]) -> i32 {
    let file = root.join(".git").join("QA_COMMIT_MESSAGE");
    std::fs::write(&file, message).unwrap();
    let mut command = Command::new("git");
    command.current_dir(root).args([
        "-c",
        "user.name=qa",
        "-c",
        "user.email=qa@example.invalid",
        "commit",
        "--quiet",
        "--allow-empty",
        "-F",
    ]);
    command.arg(&file);
    for name in LOCALE_VARIABLES {
        command.env_remove(name);
    }
    let output = command.output().unwrap();
    output
        .status
        .code()
        .unwrap_or_else(|| panic!("git commit was killed: {output:?}"))
}

/// Returns the number of commits on the current branch (0 before the first one).
fn commit_count(root: &Path) -> usize {
    let output = Command::new("git")
        .current_dir(root)
        .args(["rev-list", "--count", "--all"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .unwrap()
}

/// Row 18-1 and E6 through a real `git commit`: with the hook installed by `hooks::run` (what
/// `make hooks` runs), every AI credit of §17.7 is refused and creates no commit, while a clean
/// message and a human co-author are committed.
#[test]
fn real_commits_with_an_ai_credit_are_refused_and_create_nothing() {
    let root = repo_with_project_hook("hook-real-commit");
    for message in AI_CREDITS {
        assert_eq!(git_commit_exit(&root, message.as_bytes()), 1, "{message}");
    }
    assert_eq!(commit_count(&root), 0, "a refused commit was created");
    assert_eq!(git_commit_exit(&root, b"Add the parser\n"), 0);
    assert_eq!(
        git_commit_exit(
            &root,
            b"Fix the parser\n\nCo-authored-by: Ada Lovelace <ada@example.org>\n"
        ),
        0
    );
    for message in AI_CREDITS {
        assert_eq!(git_commit_exit(&root, message.as_bytes()), 1, "{message}");
    }
    assert_eq!(commit_count(&root), 2);
    std::fs::remove_dir_all(&root).unwrap();
}
