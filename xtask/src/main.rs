//! `cargo xtask <command>`: entry point of the developer task runner.
//!
//! Parses the command line with clap (usage errors exit with code 2), switches the working
//! directory to the workspace root, then dispatches to the library modules. Any error is printed
//! on stderr as `error: <message with its context chain>` and the process exits with code 1,
//! except `docs`, which forwards the exit code of the documentation check script.

use std::process::ExitCode;

use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};
use xtask::{bench, ci, docs, hooks, locate};

/// Command line of `cargo xtask`.
#[derive(Debug, Parser)]
#[command(
    name = "xtask",
    bin_name = "cargo xtask",
    version,
    about = "VisualAssembler developer task runner",
    arg_required_else_help = true
)]
struct Cli {
    /// The task to run.
    #[command(subcommand)]
    command: Command,
}

/// The tasks of `cargo xtask`.
#[derive(Debug, Subcommand)]
enum Command {
    /// Build the shipped GUI once, measure startup and idle runs per renderer choice, write a
    /// JSON report.
    Bench(bench::BenchArgs),
    /// Run the CI checks: build configuration, fmt, clippy (all features, glow only, wgpu only),
    /// layering and sources, tests.
    Ci(ci::CiArgs),
    /// Regenerate AGENCEMENT.md, then verify the project documentation.
    Docs(docs::DocsArgs),
    /// Install the commit-msg git hook that rejects AI co-author mentions.
    Hooks(hooks::HooksArgs),
}

/// Parses the command line, runs the task and maps its outcome to the process exit code.
fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli.command) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

/// Runs one task from the workspace root and returns the exit code it asks for.
fn run(command: Command) -> Result<ExitCode> {
    let root = locate::workspace_root()?;
    std::env::set_current_dir(&root).with_context(|| {
        format!(
            "cannot use the workspace root {} as working directory",
            root.display()
        )
    })?;
    match command {
        Command::Bench(args) => bench::run(&root, &args).map(|()| ExitCode::SUCCESS),
        Command::Ci(args) => ci::run(&root, &args).map(|()| ExitCode::SUCCESS),
        Command::Docs(args) => docs::run(&root, &args).map(ExitCode::from),
        Command::Hooks(args) => hooks::run(&root, args.force).map(|_| ExitCode::SUCCESS),
    }
}
