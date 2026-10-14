//! `va info`: prints build information (version, target OS and architecture, registered CPUs).
//!
//! Human output (docs/specs/m0-foundations.md §5):
//! `va 0.1.0\nos: <os>\narch: <arch>\narchitectures: (none)\n`.
//! With `--json`, exactly one line of schema [`INFO_SCHEMA`]:
//! `{"schema":"va.info/1","version":"0.1.0","os":"windows","arch":"x86_64","architectures":[]}`.

use std::io::{self, Write};

use serde::Serialize;

use crate::commands::CommandError;

/// Schema identifier of the JSON document printed by `va info --json`.
pub const INFO_SCHEMA: &str = "va.info/1";

/// Build information reported by `va info`. Field order is the JSON key order.
#[derive(Debug, Serialize)]
pub struct InfoReport {
    /// Version of this binary (`CARGO_PKG_VERSION`).
    pub version: &'static str,
    /// Operating system the binary was built for (`std::env::consts::OS`).
    pub os: &'static str,
    /// CPU architecture the binary was built for (`std::env::consts::ARCH`).
    pub arch: &'static str,
    /// Names of the CPU architectures registered in this binary. Always empty in M0: no CPU is
    /// registered before M1.
    pub architectures: Vec<String>,
}

/// JSON document of `va info --json`: `schema` first, then the [`InfoReport`] fields in order.
#[derive(Serialize)]
struct InfoDocument<'a> {
    /// Always [`INFO_SCHEMA`].
    schema: &'static str,
    /// The report whose fields follow `schema` at the same level.
    #[serde(flatten)]
    report: &'a InfoReport,
}

impl InfoReport {
    /// Collects the build information of the running binary.
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            architectures: Vec::new(),
        }
    }

    /// Renders the human-readable report: the `va <version>` line, then one `key: value` line per
    /// field. An empty architecture list is shown as `(none)`; otherwise names are comma-separated.
    pub fn to_human_text(&self) -> String {
        let architectures = if self.architectures.is_empty() {
            "(none)".to_owned()
        } else {
            self.architectures.join(", ")
        };
        format!(
            "va {}\nos: {}\narch: {}\narchitectures: {}\n",
            self.version, self.os, self.arch, architectures
        )
    }

    /// Writes the report to `out` as a single JSON line of schema [`INFO_SCHEMA`], ending in `\n`.
    pub fn write_json_line<W: Write>(&self, mut out: W) -> io::Result<()> {
        let document = InfoDocument {
            schema: INFO_SCHEMA,
            report: self,
        };
        // serde_json converts its error into io::Error, keeping the original I/O error if any.
        serde_json::to_writer(&mut out, &document)?;
        out.write_all(b"\n")
    }
}

/// Runs `va info`: prints the report on stdout, as one JSON line when `json` is true, and flushes
/// stdout so that a write failure is reported instead of being lost at process exit.
pub fn run(json: bool) -> Result<(), CommandError> {
    let report = InfoReport::current();
    let mut stdout = io::stdout().lock();
    let written = if json {
        report.write_json_line(&mut stdout)
    } else {
        stdout.write_all(report.to_human_text().as_bytes())
    };
    written
        .and_then(|()| stdout.flush())
        .map_err(CommandError::Stdout)
}
