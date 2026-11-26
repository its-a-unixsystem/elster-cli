// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

mod reference;
mod schemas;

use std::{any::TypeId, borrow::Cow, io::Write, path::PathBuf};

use clap::{Arg, ArgAction, Command, CommandFactory};
use schemars::{JsonSchema, Schema};
use serde::Serialize;

use crate::{Cli, error::Failure, kontoabfrage, postfach, write_json};
use reference::{Code, ExitCode, KontoabfrageCode, Setting};

#[derive(Serialize, JsonSchema)]
struct Argument<'a> {
    id: &'a str,
    long: Option<&'a str>,
    short: Option<char>,
    position: Option<usize>,
    help: Option<String>,
    value_type: &'static str,
    value_names: Vec<&'a str>,
    possible_values: Vec<String>,
    default_values: Vec<Cow<'a, str>>,
    required: bool,
    repeatable: bool,
    conflicts_with: Vec<&'a str>,
}

impl<'a> Argument<'a> {
    fn from_arg(command: &'a Command, arg: &'a Arg, path: &str) -> Self {
        let parser = arg.get_value_parser();
        let id = parser.type_id();
        let possible_values: Vec<_> = parser
            .possible_values()
            .into_iter()
            .flatten()
            .filter(|value| !value.is_hide_set())
            .map(|value| value.get_name().to_owned())
            .collect();
        let value_type = if !arg.get_action().takes_values() {
            "boolean"
        } else if !possible_values.is_empty() {
            "enum"
        } else if id == TypeId::of::<PathBuf>() {
            "path"
        } else if id == TypeId::of::<time::Date>() {
            "date"
        } else if id == TypeId::of::<u16>() {
            "integer"
        } else {
            "string"
        };
        Self {
            id: arg.get_id().as_str(),
            long: arg.get_long(),
            short: arg.get_short(),
            position: arg.get_index(),
            help: arg
                .get_long_help()
                .or_else(|| arg.get_help())
                .map(ToString::to_string),
            value_type,
            value_names: arg
                .get_value_names()
                .into_iter()
                .flatten()
                .map(|value| value.as_str())
                .collect(),
            possible_values,
            default_values: effective_defaults(arg, path),
            required: arg.is_required_set(),
            repeatable: matches!(arg.get_action(), ArgAction::Append | ArgAction::Count),
            conflicts_with: command
                .get_arg_conflicts_with(arg)
                .into_iter()
                .map(|other| other.get_id().as_str())
                .collect(),
        }
    }
}

fn effective_defaults<'a>(arg: &'a Arg, path: &str) -> Vec<Cow<'a, str>> {
    match (path, arg.get_id().as_str()) {
        ("postfach status" | "postfach anfrage", id)
            if id == postfach::DEFAULT_RESTRICTION.as_str() =>
        {
            vec![Cow::Borrowed("true")]
        }
        ("kontoabfrage zahlungen", "steuerarten") => {
            vec![Cow::Borrowed(kontoabfrage::DEFAULT_PAYMENT_STEUERART.as_str())]
        }
        _ => arg
            .get_default_values()
            .iter()
            .map(|value| value.to_string_lossy())
            .collect(),
    }
}

#[derive(Serialize, JsonSchema)]
struct ExclusiveGroup<'a> {
    name: &'a str,
    arguments: Vec<&'a str>,
    required: bool,
}

#[derive(Serialize, JsonSchema)]
struct CommandDescription<'a> {
    path: String,
    about: Option<String>,
    arguments: Vec<Argument<'a>>,
    exclusive_groups: Vec<ExclusiveGroup<'a>>,
    stdout_format: Option<&'static str>,
    #[schemars(with = "Option<std::collections::BTreeMap<String, serde_json::Value>>")]
    stdout_schema: Option<Schema>,
}

#[derive(Serialize, JsonSchema)]
struct Description<'a> {
    version: &'static str,
    test: bool,
    profile: Option<String>,
    warnings: Vec<crate::config::Warning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    trace: Option<Vec<crate::eric::TraceEvent>>,
    global_arguments: Vec<Argument<'a>>,
    global_exclusive_groups: [ExclusiveGroup<'static>; 1],
    commands: Vec<CommandDescription<'a>>,
    settings: &'static [Setting],
    exit_codes: &'static [ExitCode],
    error_codes: &'static [Code],
    warning_codes: &'static [Code],
    bereitstellungsdatenarten: &'static [&'static str],
    steuerarten: Vec<&'static str>,
    kontoabfrage_result_codes: &'static [KontoabfrageCode],
    #[schemars(with = "std::collections::BTreeMap<String, serde_json::Value>")]
    stderr_schema: Schema,
}

fn collect<'a>(command: &'a Command, path: String, out: &mut Vec<CommandDescription<'a>>) {
    let stdout_schema = schemas::stdout(&path);
    let stdout_format = if path == "skill" {
        Some("markdown")
    } else if visible_commands(command).next().is_none() {
        Some("json")
    } else {
        None
    };
    let arguments = command
        .get_arguments()
        .filter(|arg| !arg.is_global_set() && !arg.is_hide_set())
        .map(|arg| Argument::from_arg(command, arg, &path))
        .collect();
    let exclusive_groups = command
        .get_groups()
        .filter(|group| group.get_args().count() > 1 && !(*group).clone().is_multiple())
        .map(|group| ExclusiveGroup {
            name: group.get_id().as_str(),
            arguments: group.get_args().map(|id| id.as_str()).collect(),
            required: group.is_required_set(),
        })
        .collect();
    let index = out.len();
    out.push(CommandDescription {
        path,
        about: command
            .get_long_about()
            .or_else(|| command.get_about())
            .map(ToString::to_string),
        arguments,
        exclusive_groups,
        stdout_format,
        stdout_schema,
    });
    for child in visible_commands(command) {
        let child_path = format!("{} {}", out[index].path, child.get_name());
        collect(child, child_path, out);
    }
}

fn visible_commands(command: &Command) -> impl Iterator<Item = &Command> {
    command
        .get_subcommands()
        .filter(|child| !child.is_hide_set() && child.get_name() != "help")
}

pub(super) fn run(path: &[String], verbose: bool, stdout: impl Write) -> Result<(), Failure> {
    let mut root = Cli::command();
    root.build();
    let mut commands = Vec::new();
    if path.is_empty() {
        for child in visible_commands(&root) {
            collect(child, child.get_name().to_owned(), &mut commands);
        }
    } else {
        let mut selected = &root;
        let mut canonical = String::new();
        for name in path.iter().flat_map(|part| part.split_ascii_whitespace()) {
            selected = visible_commands(selected)
                .find(|child| child.get_name() == name)
                .ok_or_else(|| {
                    Failure::new(
                        2,
                        "usage_error",
                        format!("Unknown command path: {:?}", path.join(" ")),
                    )
                    .help("Run 'kiecli describe' to list available commands.")
                })?;
            if !canonical.is_empty() {
                canonical.push(' ');
            }
            canonical.push_str(selected.get_name());
        }
        if canonical.is_empty() {
            return Err(Failure::new(2, "usage_error", "Expected a command path"));
        }
        collect(selected, canonical, &mut commands);
    }
    let description = Description {
        version: env!("CARGO_PKG_VERSION"),
        test: false,
        profile: None,
        warnings: Vec::new(),
        trace: verbose.then(Vec::new),
        global_arguments: root
            .get_arguments()
            .filter(|arg| !arg.is_hide_set())
            .map(|arg| Argument::from_arg(&root, arg, ""))
            .collect(),
        // Also enforced across different levels of the CLI, after clap parses the flags.
        global_exclusive_groups: [ExclusiveGroup {
            name: "pin_source",
            arguments: vec!["pin_file", "pin_command"],
            required: false,
        }],
        commands,
        settings: reference::SETTINGS,
        exit_codes: reference::EXIT_CODES,
        error_codes: reference::ERROR_CODES,
        warning_codes: reference::WARNING_CODES,
        bereitstellungsdatenarten: postfach::DATENARTEN,
        steuerarten: kontoabfrage::STEUERARTEN
            .iter()
            .map(|kind| kind.as_str())
            .collect(),
        kontoabfrage_result_codes: reference::KONTOABFRAGE_CODES,
        stderr_schema: schemas::stderr(),
    };
    write_json(stdout, &description)
        .map_err(|err| Failure::new(1, "output_failed", err.to_string()))
}
