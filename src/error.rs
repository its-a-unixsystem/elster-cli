// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::io::{self, Write};

use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct Report<'a> {
    pub(crate) error: &'a Failure,
}

#[derive(Serialize)]
pub(crate) struct Failure {
    #[serde(skip)]
    pub(crate) exit: u8,
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notes: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    help: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cause: Option<Box<[EricCause; 1]>>,
    retry: bool,
}

#[derive(Serialize)]
struct EricCause {
    source: &'static str,
    code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

impl Failure {
    pub(crate) fn new(exit: u8, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            exit,
            code,
            message: message.into(),
            notes: Vec::new(),
            help: None,
            cause: None,
            retry: false,
        }
    }

    pub(crate) fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub(crate) fn note(mut self, note: &'static str) -> Self {
        self.notes.push(note);
        self
    }

    pub(crate) fn eric_cause(mut self, code: i32, text: Option<String>) -> Self {
        self.cause = Some(Box::new([EricCause {
            source: "eric",
            code,
            text,
        }]));
        self
    }

    pub(crate) fn write_text(&self, mut out: impl Write) -> io::Result<()> {
        // clap includes its own prefix and a trailing newline in usage errors.
        let message = self
            .message
            .strip_prefix("error: ")
            .unwrap_or(&self.message);
        writeln!(out, "error[{}]: {}", self.code, message.trim_end())?;
        for note in &self.notes {
            writeln!(out, "= note: {note}")?;
        }
        if let Some(help) = &self.help {
            writeln!(out, "= help: {help}")?;
        }
        if let Some([cause]) = self.cause.as_deref() {
            write!(out, "= note: {} returned {}", cause.source, cause.code)?;
            if let Some(text) = &cause.text {
                write!(out, ": {text}")?;
            }
            writeln!(out)?;
        }
        Ok(())
    }
}
