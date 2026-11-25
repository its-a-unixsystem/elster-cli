// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::env;
use std::ffi::{CStr, OsStr, OsString};
use std::fs::OpenOptions;
use std::io::{self, Read};
use std::os::fd::AsFd;
use std::os::unix::{
    ffi::OsStringExt,
    fs::{OpenOptionsExt, PermissionsExt},
    process::CommandExt,
};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::fs::OFlags;
use rustix::process::{Pid, Signal, kill_process_group};
use zeroize::Zeroizing;

use crate::{config::Warning, error::Failure};

type Secret = Zeroizing<Vec<u8>>;

const MAX_PIN_BYTES: usize = 4096;
const MAX_DIAGNOSTIC_BYTES: usize = 8192;

pub(crate) enum Source {
    Env(Secret),
    File(PathBuf),
    Command(OsString),
}

impl Source {
    pub(crate) fn environment() -> Option<Self> {
        env::var_os("KIECLI_PIN")
            .map(|value| Self::Env(Zeroizing::new(value.into_vec())))
            .or_else(|| env::var_os("KIECLI_PIN_FILE").map(|path| Self::File(path.into())))
            .or_else(|| env::var_os("KIECLI_PIN_COMMAND").map(Self::Command))
    }

    pub(crate) fn channel(&self) -> &'static str {
        match self {
            Self::Env(_) => "env",
            Self::File(_) => "file",
            Self::Command(_) => "command",
        }
    }

    pub(crate) fn read(self, warnings: &mut Vec<Warning>) -> Result<Pin, Failure> {
        let strip = !matches!(self, Self::Env(_));
        let mut bytes = match self {
            Self::Env(bytes) => bytes,
            Self::File(path) => {
                let failure = |err: io::Error| {
                    Failure::new(
                        3,
                        "pin_file_unreadable",
                        format!("Cannot read PIN file '{}': {err}", path.display()),
                    )
                };
                let mut file = OpenOptions::new()
                    .read(true)
                    .custom_flags(OFlags::NONBLOCK.bits() as i32)
                    .open(&path)
                    .map_err(failure)?;
                let metadata = file.metadata().map_err(failure)?;
                if !metadata.is_file() {
                    return Err(failure(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "not a regular file",
                    )));
                }
                if metadata.permissions().mode() & 0o044 != 0 {
                    warnings.push(Warning::new(
                        "pin_file_permissions",
                        "The PIN file is readable by group or others",
                    ));
                }
                let mut bytes = Zeroizing::new(Vec::new());
                let mut buffer = Zeroizing::new([0; 256]);
                loop {
                    match file.read(&mut buffer[..]) {
                        Ok(0) => break,
                        Ok(length) => {
                            if bytes.len() + length > MAX_PIN_BYTES {
                                return Err(failure(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "PIN file is too large",
                                )));
                            }
                            append(&mut bytes, &buffer[..length]);
                        }
                        Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                        Err(err) => return Err(failure(err)),
                    }
                }
                bytes
            }
            Self::Command(command) => read_command(&command)?,
        };
        if strip {
            strip_newline(&mut bytes);
        }
        if bytes.is_empty() || bytes.contains(&0) {
            return Err(Failure::new(
                3,
                "pin_invalid",
                "The PIN is empty or contains a NUL byte",
            ));
        }
        append(&mut bytes, &[0]);
        Ok(Pin(bytes))
    }
}

pub(crate) struct Pin(Secret);

impl Pin {
    pub(crate) fn as_c_str(&self) -> &CStr {
        CStr::from_bytes_with_nul(&self.0).expect("PIN was validated and NUL-terminated")
    }
}

fn append(secret: &mut Secret, bytes: &[u8]) {
    let required = secret.len() + bytes.len();
    if required > secret.capacity() {
        let mut grown = Zeroizing::new(Vec::with_capacity(required.max(secret.capacity() * 2)));
        grown.extend_from_slice(secret);
        *secret = grown;
    }
    secret.extend_from_slice(bytes);
}

fn strip_newline(bytes: &mut Vec<u8>) {
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
    }
}

fn read_command(command: &OsStr) -> Result<Secret, Failure> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|err| {
            Failure::new(
                3,
                "pin_command_failed",
                format!("Cannot start PIN command: {err}"),
            )
        })?;
    let mut pin = Zeroizing::new(Vec::new());
    let mut stderr = Zeroizing::new(Vec::new());
    let (code, message) = match collect(&mut child, deadline, &mut pin, &mut stderr) {
        Ok(status)
            if status.success()
                && !pin.is_empty()
                && !matches!(pin.as_slice(), b"\n" | b"\r\n") =>
        {
            return Ok(pin);
        }
        Ok(status) => (
            status.code(),
            if status.success() {
                "The PIN command returned no PIN".into()
            } else {
                "The PIN command failed".into()
            },
        ),
        Err(err) => {
            if let Some(group) = Pid::from_raw(child.id() as i32) {
                let _ = kill_process_group(group, Signal::KILL);
            }
            let _ = child.wait();
            (
                None,
                if err.kind() == io::ErrorKind::TimedOut {
                    "The PIN command timed out after 30 seconds".into()
                } else {
                    format!("Cannot read PIN command output: {err}")
                },
            )
        }
    };
    strip_newline(&mut pin);
    // Without a complete PIN to redact, stderr could be the only place it appeared.
    let text = code
        .filter(|_| !pin.is_empty() && !stderr.is_empty())
        .and_then(|_| std::str::from_utf8(&pin).ok())
        .map(|pin| String::from_utf8_lossy(&stderr).replace(pin, "[redacted]"));
    Err(Failure::new(3, "pin_command_failed", message).pin_command_cause(code, text))
}

fn remaining(deadline: Instant) -> io::Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .ok_or(io::ErrorKind::TimedOut.into())
}

fn collect(
    child: &mut Child,
    deadline: Instant,
    pin: &mut Secret,
    stderr: &mut Secret,
) -> io::Result<ExitStatus> {
    let stdout = child.stdout.take().expect("piped stdout");
    let errors = child.stderr.take().expect("piped stderr");
    let mut pipes = [
        PollFd::new(&stdout, PollFlags::IN),
        PollFd::new(&errors, PollFlags::IN),
    ];
    let outputs = [pin, stderr];
    let limits = [MAX_PIN_BYTES, MAX_DIAGNOSTIC_BYTES];
    let mut open = [true, true];
    let mut buffer = Zeroizing::new([0; 4096]);
    // Drain both streams without blocking on either one or waiting on a descendant
    // that inherited the pipes after the shell exits.
    while open[0] || open[1] {
        let start = usize::from(!open[0]);
        let end = if open[1] { 2 } else { 1 };
        let timeout = remaining(deadline)?;
        let timeout = Timespec {
            tv_sec: timeout.as_secs() as _,
            tv_nsec: timeout.subsec_nanos().into(),
        };
        match poll(&mut pipes[start..end], Some(&timeout)) {
            Ok(_) => {}
            Err(rustix::io::Errno::INTR) => continue,
            Err(err) => return Err(err.into()),
        }
        for index in start..end {
            if pipes[index].revents().is_empty() {
                continue;
            }
            match rustix::io::read(pipes[index].as_fd(), &mut buffer[..]) {
                Ok(0) => open[index] = false,
                Ok(length) => {
                    if outputs[index].len() + length > limits[index] {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "PIN command output is too large",
                        ));
                    }
                    append(outputs[index], &buffer[..length]);
                }
                Err(rustix::io::Errno::INTR) => {}
                Err(err) => return Err(err.into()),
            }
        }
    }
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        std::thread::sleep(remaining(deadline)?.min(Duration::from_millis(10)));
    }
}
