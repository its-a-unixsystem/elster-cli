// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

mod certificate;
mod config;
mod eric;
mod error;
mod pin;
mod postfach;
mod kontoabfrage;
mod version;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum, error::ErrorKind};
use serde::Serialize;

use eric::install::{self, Options};
use error::Failure;

#[derive(Parser)]
#[command(version, about = "A stateless command-line client for ELSTER")]
struct Cli {
    #[arg(long, global = true, value_enum, default_value_t = Format::Json)]
    format: Format,
    #[arg(short, global = true)]
    verbose: bool,
    #[arg(long, global = true, value_name = "DIR")]
    eric_log: Option<PathBuf>,
    #[arg(long, global = true)]
    eric_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long, global = true)]
    profile: Option<String>,
    #[arg(long, global = true, overrides_with = "no_test")]
    test: bool,
    #[arg(long, global = true, overrides_with = "test")]
    no_test: bool,
    /// Path to the Taxpayer's Certificate.
    #[arg(long, global = true)]
    certificate: Option<PathBuf>,
    /// Read the PIN from this file when opening a Certificate.
    #[arg(long, global = true)]
    pin_file: Option<PathBuf>,
    /// Run this shell command to obtain the PIN when opening a Certificate.
    #[arg(long, global = true)]
    pin_command: Option<String>,
    /// The five-digit HerstellerID.
    #[arg(long, global = true)]
    hersteller_id: Option<String>,
    /// The Taxpayer's Steuernummer.
    #[arg(long, global = true)]
    steuernummer: Option<String>,
    /// Bundesland for a Bescheid-format Steuernummer.
    #[arg(long, global = true)]
    bundesland: Option<String>,
    #[arg(long, global = true)]
    mit_xml: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
enum Format {
    #[default]
    Json,
    Text,
}

#[derive(Subcommand)]
enum Command {
    /// Install or inspect ERiC and Otto.
    Eric {
        #[command(subcommand)]
        command: EricCommand,
    },
    /// Inspect local configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Inspect a local ELSTER Certificate.
    Zertifikat {
        #[command(subcommand)]
        command: CertificateCommand,
    },
    /// Read the Taxpayer's Postfach.
    Postfach {
        #[command(subcommand)]
        command: postfach::Command,
    },
    Kontoabfrage {
        #[command(subcommand)]
        command: kontoabfrage::Command,
    },
}

#[derive(Subcommand)]
enum EricCommand {
    /// Test-load ERiC and Otto without contacting ELSTER.
    Status,
    /// Download, verify and activate ERiC and Otto.
    Install(Options),
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Show effective settings and their sources without using credentials.
    Show,
}

#[derive(Subcommand)]
enum CertificateCommand {
    /// Open a Certificate with its PIN without contacting ELSTER.
    Status,
}

#[derive(Serialize)]
struct Status<'a> {
    #[serde(serialize_with = "eric::serialize_path")]
    eric_dir: &'a std::path::Path,
    #[serde(flatten)]
    probe: eric::Probe,
    supported_majors: &'static [u16],
    test_load: bool,
    test: bool,
    profile: Option<String>,
    warnings: Vec<config::Warning>,
}

fn run(cli: &Cli, mut stdout: impl Write) -> Result<(), Failure> {
    // Both flags can be provided at different levels of the subcommand tree.
    if cli.pin_file.is_some() && cli.pin_command.is_some() {
        return Err(Failure::new(
            2,
            "usage_error",
            "Use either --pin-file or --pin-command, not both",
        ));
    }
    match &cli.command {
        Command::Eric {
            command: EricCommand::Status,
        } => {
            let settings = config::resolve(cli)?;
            let (dir, lookup_error) = match eric::installed_dir(&settings.eric_dir) {
                Ok(dir) => (dir, None),
                Err(err) => (None, Some(err)),
            };
            let dir = dir.as_deref().unwrap_or(&settings.eric_dir);
            let mut probe = eric::Probe::new(dir, cli.verbose);
            let outcome =
                lookup_error.map_or_else(|| probe.check(dir, cli.eric_log.as_deref()), Err);
            if outcome.as_ref().is_err_and(|err| err.exit != 4) {
                return outcome;
            }
            let status = Status {
                eric_dir: dir,
                probe,
                supported_majors: eric::SUPPORTED_MAJORS,
                test_load: outcome.is_ok(),
                test: settings.test,
                profile: settings.profile,
                warnings: settings.warnings,
            };
            let result = match cli.format {
                Format::Json => write_json(&mut stdout, &status),
                Format::Text => status.write_text(&mut stdout),
            };
            result.map_err(|err| Failure::new(1, "output_failed", err.to_string()))?;
            outcome
        }
        Command::Eric {
            command: EricCommand::Install(options),
        } => {
            let package = options.package()?;
            let settings = config::resolve(cli)?;
            let installed = install::run(package, settings, cli.eric_log.as_deref(), cli.verbose)?;
            match cli.format {
                Format::Json => write_json(&mut stdout, &installed),
                Format::Text => installed.write_text(&mut stdout),
            }
            .map_err(|err| Failure::new(1, "output_failed", err.to_string()))
        }
        Command::Config {
            command: ConfigCommand::Show,
        } => config::show(cli, stdout),
        Command::Zertifikat {
            command: CertificateCommand::Status,
        } => certificate::status(cli, stdout),
        Command::Postfach { command } => postfach::run(cli, command, stdout),
        Command::Kontoabfrage { command } => kontoabfrage::run(cli, command, stdout),
    }
}

impl Status<'_> {
    fn write_text(&self, mut out: impl Write) -> io::Result<()> {
        let support = match self.probe.supported {
            Some(true) => "supported",
            Some(false) => "unsupported",
            None => "support unknown",
        };
        writeln!(
            out,
            "ERiC {} ({support}; majors: {:?})",
            self.probe.version.as_deref().unwrap_or("version unknown"),
            self.supported_majors
        )?;
        writeln!(out, "Directory: {}", self.eric_dir.display())?;
        writeln!(
            out,
            "ERiC library: {}",
            self.probe.library_paths.eric.display()
        )?;
        writeln!(
            out,
            "Otto library: {}",
            self.probe.library_paths.otto.display()
        )?;
        writeln!(
            out,
            "Terms accepted: {}; test-load passed: {}",
            self.probe.terms_accepted, self.test_load
        )?;
        writeln!(
            out,
            "Testmerker: {}; profile: {}",
            self.test,
            self.profile.as_deref().unwrap_or("none")
        )?;
        for warning in &self.warnings {
            warning.write_text(&mut out)?;
        }
        if let Some(trace) = &self.probe.trace {
            for event in trace {
                writeln!(out, "{}: {} ms", event.call, event.duration_ms)?;
            }
        }
        Ok(())
    }
}

fn write_json(mut out: impl Write, value: &impl Serialize) -> io::Result<()> {
    serde_json::to_writer(&mut out, value)?;
    writeln!(out)
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().collect();
    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            return if err.print().is_ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
        }
        Err(err) => {
            // Recover an explicit format even when another argument failed to parse.
            let mut format = Format::Json;
            let mut args = args.iter().skip(1);
            while let Some(arg) = args.next() {
                match arg.to_str() {
                    Some("--") => break,
                    Some("--format") => match args.next().and_then(|arg| arg.to_str()) {
                        Some("text") => format = Format::Text,
                        Some("json") => format = Format::Json,
                        _ => {}
                    },
                    Some("--format=text") => format = Format::Text,
                    Some("--format=json") => format = Format::Json,
                    _ => {}
                }
            }
            return report(format, Failure::new(2, "usage_error", err.to_string()));
        }
    };
    match run(&cli, io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => report(cli.format, err),
    }
}

fn report(format: Format, err: Failure) -> ExitCode {
    let mut stderr = io::stderr().lock();
    let result = match format {
        Format::Json => write_json(&mut stderr, &error::Report { error: &err }),
        Format::Text => err.write_text(&mut stderr),
    };
    if result.is_ok() {
        ExitCode::from(err.exit)
    } else {
        ExitCode::FAILURE
    }
}
