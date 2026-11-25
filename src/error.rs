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
    message: Box<str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notes: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    help: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<Box<str>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    cause: Vec<Cause>,
    retry: bool,
}

#[derive(Serialize)]
struct Cause {
    source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

impl Failure {
    pub(crate) fn new(exit: u8, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            exit,
            code,
            message: message.into().into_boxed_str(),
            notes: Vec::new(),
            help: None,
            suggestion: None,
            cause: Vec::new(),
            retry: false,
        }
    }

    pub(crate) fn code(&self) -> &'static str {
        self.code
    }

    pub(crate) fn transient(code: &'static str, message: impl Into<String>) -> Self {
        let mut failure = Self::new(6, code, message);
        failure.retry = true;
        failure
    }

    pub(crate) fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into().into_boxed_str());
        self
    }

    pub(crate) fn note(mut self, note: &'static str) -> Self {
        self.notes.push(note);
        self
    }

    pub(crate) fn suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into().into_boxed_str());
        self
    }

    pub(crate) fn cause(
        mut self,
        source: &'static str,
        code: Option<i32>,
        text: Option<String>,
    ) -> Self {
        self.cause.push(Cause { source, code, text });
        self
    }

    pub(crate) fn eric_cause(self, code: i32, text: Option<String>) -> Self {
        self.cause("eric", Some(code), text)
    }

    pub(crate) fn pin_command_cause(self, code: Option<i32>, text: Option<String>) -> Self {
        self.cause("pin_command", code, text)
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
        if let Some(suggestion) = &self.suggestion {
            writeln!(out, "= help: {suggestion}")?;
        }
        for cause in &self.cause {
            write!(out, "= note: {}", cause.source)?;
            if let Some(code) = cause.code {
                write!(out, " returned {code}")?;
            }
            if let Some(text) = &cause.text {
                write!(out, ": {text}")?;
            }
            writeln!(out)?;
        }
        Ok(())
    }
}
