// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::collections::BTreeSet;
use std::fs::{DirBuilder, File};
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};

use clap::{ArgGroup, Args};
use rustix::fs::{Mode, OFlags, mkdirat, openat};
use serde::Serialize;
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};

use super::{
    AuthenticatedEric, Cli, Einschraenkung, Failure, Filters, Operation, Scope, TraceEvent,
    anfrage::{self, Details, Nachricht},
    append_arg, bestaetigung, correction, parse_id, safe_id,
};
use crate::eric::{ObjectFetcher, serialize_path};

#[derive(Args)]
#[command(group(ArgGroup::new("selection").args(["id", "neue"]).required(true)))]
#[command(group(ArgGroup::new("confirmation").args(["bestaetigen", "ohne_bestaetigung"]).required(true)))]
pub(crate) struct Options {
    /// Collect this Postfach-Nachricht, including if already confirmed (repeatable).
    #[arg(long, value_name = "ID", value_parser = parse_id)]
    id: Vec<String>,
    /// Collect Postfach-Nachrichten that are not yet confirmed.
    #[arg(long)]
    neue: bool,
    /// Directory for the collected files.
    #[arg(long, value_name = "DIR")]
    out: PathBuf,
    /// Confirm receipt only after all files of a Nachricht are safely written.
    #[arg(long)]
    bestaetigen: bool,
    /// Do not confirm receipt; report when each Bestätigung is due.
    #[arg(long)]
    ohne_bestaetigung: bool,
    /// Include Anfrage metadata in the output (never in a separate file).
    #[arg(long)]
    mit_meta: bool,
    #[command(flatten)]
    filters: Filters,
}

impl Options {
    pub(super) fn scope(&self, cli: &Cli) -> Result<Scope, Failure> {
        Scope::from_filters(
            &self.filters,
            if self.neue {
                Einschraenkung::Neue
            } else {
                Einschraenkung::Alle
            },
            |index, name| self.correction(cli, Some((index, name)), false),
        )
    }

    pub(super) fn correction(
        &self,
        cli: &Cli,
        datenart: Option<(usize, &str)>,
        enable_test: bool,
    ) -> Option<String> {
        let mut command = String::from("kiecli postfach abholung");
        if self.neue {
            command.push_str(" --neue");
        }
        for id in &self.id {
            append_arg(&mut command, "--id", id)?;
        }
        append_arg(&mut command, "--out", self.out.to_str()?)?;
        if self.bestaetigen {
            command.push_str(" --bestaetigen");
        } else if self.ohne_bestaetigung {
            command.push_str(" --ohne-bestaetigung");
        }
        if self.mit_meta {
            command.push_str(" --mit-meta");
        }
        correction(cli, command, Some(&self.filters), datenart, enable_test)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Vollstaendig,
    Teilweise,
    Fehlgeschlagen,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Self::Vollstaendig => "vollstaendig",
            Self::Teilweise => "teilweise",
            Self::Fehlgeschlagen => "fehlgeschlagen",
        }
    }
}

#[derive(Serialize)]
pub(super) struct Collected {
    id: String,
    status: Status,
    #[serde(with = "time::serde::rfc3339::option")]
    abgeholt_am: Option<OffsetDateTime>,
    files: Vec<WrittenFile>,
    bestaetigt: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    bestaetigung_faellig_bis: Option<OffsetDateTime>,
    fehler: Option<Failure>,
    #[serde(flatten)]
    details: Option<Details>,
}

impl Collected {
    fn new(id: String) -> Self {
        Self {
            id,
            status: Status::Vollstaendig,
            abgeholt_am: None,
            files: Vec::new(),
            bestaetigt: false,
            bestaetigung_faellig_bis: None,
            fehler: None,
            details: None,
        }
    }

    pub(super) fn pending(&self) -> bool {
        self.bestaetigung_faellig_bis.is_some()
    }

    fn ready_to_confirm(&self) -> bool {
        self.status == Status::Vollstaendig && self.abgeholt_am.is_some()
    }

    pub(super) fn write_text(&self, mut out: impl Write) -> io::Result<()> {
        writeln!(out, "{}: {}", self.id.escape_debug(), self.status.as_str())?;
        for file in &self.files {
            writeln!(
                out,
                "  {} ({} bytes)",
                file.path.to_string_lossy().escape_debug(),
                file.groesse
            )?;
        }
        if let Some(deadline) = self.bestaetigung_faellig_bis {
            writeln!(out, "  Confirm by {deadline}")?;
        } else if self.bestaetigt {
            writeln!(out, "  Confirmed")?;
        }
        if let Some(failure) = &self.fehler {
            failure.write_text(&mut out)?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Role {
    Datenpaket,
    Anhang,
}

#[derive(Serialize)]
struct WrittenFile {
    #[serde(serialize_with = "serialize_path")]
    path: PathBuf,
    groesse: u64,
    sha256: String,
    role: Role,
    position: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding: Option<String>,
}

pub(super) fn execute(
    options: &Options,
    scope: &Scope,
    eric: &impl AuthenticatedEric,
    otto: &impl ObjectFetcher,
    hersteller_id: &str,
    test: bool,
    trace: &mut Option<Vec<TraceEvent>>,
) -> Result<Vec<Collected>, Failure> {
    let mut ids: BTreeSet<_> = options.id.iter().map(String::as_str).collect();
    // The wire Anfrage has no ID filter. For explicit IDs, select only requested
    // Nachrichten and include already confirmed ones so their files can be recovered.
    let messages = anfrage::execute_selected(scope, eric, hersteller_id, test, &ids, trace)?;
    if messages.windows(2).any(|pair| pair[0].id == pair[1].id) {
        return Err(Operation::Anfrage
            .invalid_reply()
            .note("Anfrage returned a duplicate Postfach-Nachricht ID"));
    }
    let root = if messages
        .iter()
        .any(|m| m.details.datenpaket.is_some() || !m.details.anhaenge.is_empty())
    {
        Some(Destination::root(&options.out)?)
    } else {
        None
    };
    let mut rows = Vec::with_capacity(messages.len().max(ids.len()));
    for message in messages {
        ids.remove(message.id.as_str());
        rows.push(collect(
            message,
            root.as_ref(),
            options.mit_meta,
            otto,
            trace,
        ));
    }
    if options.bestaetigen {
        let eligible: Vec<_> = rows
            .iter()
            .filter(|row| row.ready_to_confirm())
            .map(|row| row.id.as_str())
            .collect();
        let confirmed = bestaetigung::execute(&eligible, eric, hersteller_id, test, trace);
        for (entry, result) in rows
            .iter_mut()
            .filter(|row| row.ready_to_confirm())
            .zip(confirmed)
        {
            entry.bestaetigt = result.bestaetigt;
            if entry.bestaetigt {
                entry.bestaetigung_faellig_bis = None;
            } else {
                entry.fehler = result.into_failure();
            }
        }
    }
    for id in ids {
        let mut entry = Collected::new(id.to_owned());
        entry.status = Status::Fehlgeschlagen;
        entry.fehler = Some(Failure::new(
            5,
            "postfach_nachricht_not_found",
            "Anfrage did not find this Postfach-Nachricht",
        ));
        rows.push(entry);
    }
    rows.sort_unstable_by(|a, b| a.id.cmp(&b.id));
    Ok(rows)
}

pub(super) fn outcome(rows: Vec<Collected>) -> Result<(), Failure> {
    if rows.iter().all(|row| row.fehler.is_none()) {
        return Ok(());
    }
    if rows.iter().any(|row| row.status != Status::Fehlgeschlagen) {
        return Err(Failure::new(
            7,
            "abholung_partial",
            "Some requested files were not collected or some Bestätigungen failed",
        ));
    }
    Err(rows
        .into_iter()
        .find_map(|row| row.fehler)
        .expect("at least one failed Abholung"))
}

fn collect(
    message: Nachricht,
    root: Option<&Destination>,
    include_meta: bool,
    otto: &impl ObjectFetcher,
    trace: &mut Option<Vec<TraceEvent>>,
) -> Collected {
    let Nachricht { id, details } = message;
    let mut entry = Collected::new(id);
    if !safe_id(&entry.id) {
        entry.fehler = Some(
            Operation::Anfrage
                .invalid_reply()
                .note("Anfrage returned an unsafe Postfach-Nachricht ID"),
        );
    } else if details.datenpaket.is_none() && details.anhaenge.is_empty() {
        entry.fehler = Some(Failure::new(
            5,
            "postfach_nachricht_no_objects",
            "Anfrage returned no files for this Postfach-Nachricht",
        ));
    } else {
        let dir = root
            .expect("output root for messages with files")
            .child(&entry.id);
        match dir {
            Err(failure) => entry.fehler = Some(failure),
            Ok(dir) => {
                if let Some(package) = &details.datenpaket {
                    let result = dir.fetch(
                        Object::Datenpaket(package),
                        otto,
                        trace,
                        &mut entry.abgeholt_am,
                    );
                    record(result, &mut entry);
                }
                if !details.anhaenge.is_empty() {
                    match dir.child("anhaenge") {
                        Ok(attachments) => {
                            for attachment in &details.anhaenge {
                                let result = attachments.fetch(
                                    Object::Anhang(attachment),
                                    otto,
                                    trace,
                                    &mut entry.abgeholt_am,
                                );
                                record(result, &mut entry);
                            }
                        }
                        Err(failure) => record(Err(failure), &mut entry),
                    }
                }
            }
        }
    }
    if let Some(started) = entry.abgeholt_am {
        entry.bestaetigung_faellig_bis = Some(started + Duration::hours(24));
    }
    if entry.fehler.is_some() {
        entry.status = if entry.abgeholt_am.is_some() {
            Status::Teilweise
        } else {
            Status::Fehlgeschlagen
        };
    }
    if include_meta {
        entry.details = Some(details);
    }
    entry
}

fn record(result: Result<WrittenFile, Failure>, entry: &mut Collected) {
    match result {
        Ok(file) => entry.files.push(file),
        Err(failure) if entry.fehler.is_none() => entry.fehler = Some(failure),
        Err(_) => {}
    }
}

struct Destination {
    dir: File,
    path: PathBuf,
}

impl Destination {
    fn root(path: &Path) -> Result<Self, Failure> {
        let fail = |err| write_failure(path, err);
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)
            .map_err(fail)?;
        let dir = File::from(
            rustix::fs::open(
                path,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(io::Error::from)
            .map_err(fail)?,
        );
        require_private_writes(&dir, path)?;
        // A previous interrupted run may have created any of these parents
        // without syncing its directory entry. Sync the chain on retries too.
        let absolute = path.canonicalize().map_err(fail)?;
        for parent in absolute.ancestors().skip(1) {
            File::open(parent)
                .and_then(|parent| parent.sync_all())
                .map_err(fail)?;
        }
        Ok(Self {
            dir,
            path: path.to_owned(),
        })
    }

    fn child(&self, name: &str) -> Result<Self, Failure> {
        let path = self.path.join(name);
        let fail = |err| write_failure(&path, err);
        match mkdirat(&self.dir, name, Mode::RWXU) {
            Ok(()) | Err(rustix::io::Errno::EXIST) => {}
            Err(err) => return Err(fail(err.into())),
        }
        let dir = openat(
            &self.dir,
            name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(io::Error::from)
        .map_err(fail)?;
        let dir = File::from(dir);
        require_private_writes(&dir, &path)?;
        self.dir.sync_all().map_err(fail)?;
        Ok(Self { dir, path })
    }

    fn fetch(
        &self,
        object: Object<'_>,
        otto: &impl ObjectFetcher,
        trace: &mut Option<Vec<TraceEvent>>,
        started: &mut Option<OffsetDateTime>,
    ) -> Result<WrittenFile, Failure> {
        // Keep the destination pinned through rename and fsync, even if a path
        // component is replaced after we opened it. Postfach children never follow symlinks.
        let dir = PathBuf::from(format!("/proc/self/fd/{}", self.dir.as_raw_fd()));
        let mut temp =
            tempfile::NamedTempFile::new_in(&dir).map_err(|err| write_failure(&self.path, err))?;
        let (id, transport_size) = object.reference();
        let at = OffsetDateTime::now_utc();
        let mut writer = ObjectWriter {
            file: temp.as_file_mut(),
            started,
            at,
            size: 0,
            digest: Sha256::new(),
            prefix: 0,
            xml: matches!(&object, Object::Anhang(_)).then_some(false),
        };
        otto.fetch(id, transport_size, &mut writer, trace)?;
        writer.started.get_or_insert(at);
        let (size, digest, xml) = (
            writer.size,
            writer.digest.finalize(),
            writer.xml.unwrap_or(false),
        );
        let (filename, role, position, encoding) = object.destination(xml);
        let path = self.path.join(&filename);
        let fail = |err| write_failure(&path, err);
        temp.as_file().sync_all().map_err(fail)?;
        temp.persist(dir.join(filename))
            .map_err(|err| fail(err.error))?;
        self.dir.sync_all().map_err(fail)?;
        Ok(WrittenFile {
            path,
            groesse: size,
            sha256: format!("{digest:x}"),
            role,
            position,
            encoding,
        })
    }
}

fn require_private_writes(dir: &File, path: &Path) -> Result<(), Failure> {
    let metadata = dir.metadata().map_err(|err| write_failure(path, err))?;
    if metadata.uid() != rustix::process::geteuid().as_raw() || metadata.mode() & 0o022 != 0 {
        return Err(Failure::new(
            1,
            "postfach_write_failed",
            format!(
                "Postfach output directory '{}' must be owned by the current user and not writable by group or others",
                path.to_string_lossy().escape_debug()
            ),
        ));
    }
    Ok(())
}

fn write_failure(path: &Path, err: io::Error) -> Failure {
    Failure::new(
        1,
        "postfach_write_failed",
        format!(
            "Cannot store Postfach files at '{}': {err}",
            path.to_string_lossy().escape_debug()
        ),
    )
}

struct ObjectWriter<'a> {
    file: &'a mut File,
    started: &'a mut Option<OffsetDateTime>,
    at: OffsetDateTime,
    size: u64,
    digest: Sha256,
    prefix: usize,
    xml: Option<bool>,
}

impl Write for ObjectWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // A local write failure still leaves a Bestätigung obligation when
        // Otto already delivered data, even if the temporary file is removed.
        self.started.get_or_insert(self.at);
        let written = self.file.write(bytes)?;
        let bytes = &bytes[..written];
        self.digest.update(bytes);
        self.size += written as u64;
        if self.xml.is_none() {
            const BOM: &[u8] = b"\xef\xbb\xbf";
            for &byte in bytes {
                if BOM.get(self.prefix) == Some(&byte) {
                    self.prefix += 1;
                } else if matches!(self.prefix, 1 | 2) || !byte.is_ascii_whitespace() {
                    self.xml = Some(byte == b'<' && !matches!(self.prefix, 1 | 2));
                    break;
                } else {
                    self.prefix = BOM.len();
                }
            }
        }
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

enum Object<'a> {
    Datenpaket(&'a anfrage::Datenpaket),
    Anhang(&'a anfrage::Anhang),
}

impl Object<'_> {
    fn reference(&self) -> (&str, u64) {
        match self {
            Self::Datenpaket(object) => (&object.objektid, object.groesse),
            Self::Anhang(object) => (&object.objektid, object.groesse),
        }
    }

    fn destination(self, xml: bool) -> (String, Role, Option<usize>, Option<String>) {
        match self {
            Self::Datenpaket(object) => (
                if xml {
                    "datenpaket.xml"
                } else {
                    "datenpaket.txt"
                }
                .to_owned(),
                Role::Datenpaket,
                None,
                Some(object.encoding.clone()),
            ),
            Self::Anhang(object) => {
                let mut name = format!("{:02}-", object.position);
                if let Some(filename) = object.dateiname.as_deref().filter(|name| !name.is_empty())
                {
                    for ch in filename.chars() {
                        if name.len() + ch.len_utf8() > 240 {
                            break;
                        }
                        name.push(match ch {
                            '/' | '\\' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' => '_',
                            ch if ch.is_control() => '_',
                            ch => ch,
                        });
                    }
                } else {
                    name.push_str("anhang");
                    let extension = &object.dateityp;
                    if !extension.is_empty()
                        && extension.len() <= 8
                        && extension.bytes().all(|b| b.is_ascii_alphanumeric())
                    {
                        use std::fmt::Write as _;
                        write!(name, ".{}", extension.to_ascii_lowercase()).unwrap();
                    }
                }
                (name, Role::Anhang, Some(object.position), None)
            }
        }
    }
}

#[cfg(test)]
mod tests;
