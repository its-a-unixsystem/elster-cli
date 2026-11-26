// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::fs::{self, File, Permissions};
use std::io::{self, BufReader, ErrorKind, Read, Seek, Write};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use clap::Args;
use schemars::JsonSchema;
use serde::Serialize;
use sha2::{Digest, Sha256};
use ureq::{Agent, Body, http::StatusCode};
use url::Url;
use zip::ZipArchive;

use super::{
    ACTIVE_LINK, INSTALL_HELP, PACKAGE_NAME, Probe, SUPPORTED_MAJORS, TERMS_MARKER, TraceEvent,
    VERSIONS_DIR, serialize_path,
};
use crate::{config, error::Failure};

const DEFAULT_VERSION: &str = "44.3.6.0";
const DEFAULT_SHA256: &str = "e36df8b02045ff3449eb23e086a39f5455b709c967e708eae44f27099ce4de95";
const DOWNLOAD_BASE: &str = "https://download.elster.de/download/eric";
const PRIVACY_URL: &str =
    "https://www.elster.de/eportal/helpGlobal?themaGlobal=datenschutz_public#ds_steuerverw";
const LICENSE: &str = "lizenz.pdf";
const MAX_DOWNLOAD: usize = 1024 * 1024 * 1024;
const MAX_REDIRECTS: usize = 5;

#[derive(Args)]
pub(crate) struct Options {
    /// ERiC package version to install.
    #[arg(long = "version", value_name = "VERSION", default_value = DEFAULT_VERSION)]
    version: String,
    /// Confirm that you have read and accept the ERiC terms and data-protection notice.
    #[arg(long)]
    accept_eric_terms: bool,
    /// Trusted SHA-256 for an unpinned package version.
    #[arg(long, value_name = "HEX")]
    sha256: Option<String>,
}

pub(crate) struct Package<'a> {
    version: &'a str,
    major: u16,
    sha256: &'a str,
}

impl Options {
    pub(crate) fn package(&self) -> Result<Package<'_>, Failure> {
        if !self.accept_eric_terms {
            return Err(Failure::new(
                4,
                "eric_terms_not_accepted",
                "ERiC terms acceptance is required before downloading",
            )
            .help(format!(
                "Review {LICENSE} in the official ERiC package (see the KIECLI README) and {PRIVACY_URL}, then pass --accept-eric-terms if you accept."
            )));
        }
        let mut parts = self.version.split('.');
        let invalid = || {
            Failure::new(
                2,
                "eric_version_invalid",
                "--version must be four dot-separated numbers without leading zeros",
            )
        };
        let mut numbers = [0_u16; 4];
        for number in &mut numbers {
            let part = parts.next().ok_or_else(invalid)?;
            if part.is_empty()
                || (part.len() > 1 && part.starts_with('0'))
                || !part.bytes().all(|byte| byte.is_ascii_digit())
            {
                return Err(invalid());
            }
            *number = part.parse().map_err(|_| invalid())?;
        }
        if parts.next().is_some() {
            return Err(invalid());
        }
        let major = numbers[0];
        if !SUPPORTED_MAJORS.contains(&major) {
            return Err(Failure::new(
                4,
                "eric_version_unsupported",
                format!(
                    "ERiC {} is not supported; supported majors: {SUPPORTED_MAJORS:?}",
                    self.version
                ),
            )
            .help(INSTALL_HELP));
        }
        let supplied = self.sha256.as_deref();
        if supplied
            .is_some_and(|hex| hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err(Failure::new(
                2,
                "eric_sha256_invalid",
                "--sha256 must contain exactly 64 hexadecimal digits",
            ));
        }
        let sha256 = if self.version == DEFAULT_VERSION {
            if supplied.is_some_and(|hex| !hex.eq_ignore_ascii_case(DEFAULT_SHA256)) {
                return Err(Failure::new(
                    4,
                    "eric_sha256_conflict",
                    "--sha256 cannot override the pinned ERiC package checksum",
                ));
            }
            DEFAULT_SHA256
        } else {
            supplied.ok_or_else(|| {
                Failure::new(
                    4,
                    "eric_sha256_required",
                    format!("No checksum is pinned for ERiC {}", self.version),
                )
                .help("Supply a trusted checksum with --sha256 or install the default version.")
            })?
        };
        Ok(Package {
            version: &self.version,
            major,
            sha256,
        })
    }
}

#[derive(Serialize, JsonSchema)]
pub(crate) struct Installed<'a> {
    version: &'a str,
    #[serde(serialize_with = "serialize_path")]
    #[schemars(with = "String")]
    eric_dir: PathBuf,
    #[serde(serialize_with = "serialize_path")]
    #[schemars(with = "String")]
    license: PathBuf,
    data_protection_notice: &'static str,
    test: bool,
    profile: Option<String>,
    warnings: Vec<config::Warning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    trace: Option<Vec<TraceEvent>>,
}

impl Installed<'_> {
    pub(crate) fn write_text(&self, mut out: impl Write) -> io::Result<()> {
        writeln!(
            out,
            "Installed ERiC {} in {}",
            self.version,
            self.eric_dir.display()
        )?;
        writeln!(out, "Licence: {}", self.license.display())?;
        writeln!(
            out,
            "Data-protection notice: {}",
            self.data_protection_notice
        )?;
        for warning in &self.warnings {
            warning.write_text(&mut out)?;
        }
        if let Some(trace) = &self.trace {
            for event in trace {
                writeln!(out, "{}: {} ms", event.call, event.duration_ms)?;
            }
        }
        Ok(())
    }
}

pub(crate) fn run<'a>(
    package: Package<'a>,
    settings: config::Settings,
    log_dir: Option<&Path>,
    verbose: bool,
) -> Result<Installed<'a>, Failure> {
    let version = package.version;
    let mut body = download(&package)?;
    let (eric_dir, trace) = install_from(
        &package,
        &settings.eric_dir,
        body.as_reader(),
        log_dir,
        verbose,
    )?;
    Ok(Installed {
        version,
        license: eric_dir.join(LICENSE),
        eric_dir,
        data_protection_notice: PRIVACY_URL,
        test: settings.test,
        profile: settings.profile,
        warnings: settings.warnings,
        trace,
    })
}

fn download(package: &Package<'_>) -> Result<Body, Failure> {
    let config = Agent::config_builder()
        .https_only(true)
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(30)))
        .timeout_per_call(Some(Duration::from_secs(30 * 60)))
        .build();
    let agent: Agent = config.into();
    let mut url = Url::parse(&format!(
        "{DOWNLOAD_BASE}/eric_{}/ERiC-{}-{PACKAGE_NAME}.jar",
        package.major, package.version
    ))
    .expect("validated ERiC download URL");
    for hop in 0..=MAX_REDIRECTS {
        let response = agent.get(url.as_str()).call().map_err(|err| {
            let message = format!("Could not download ERiC: {err}");
            match err {
                ureq::Error::Io(_)
                | ureq::Error::Timeout(_)
                | ureq::Error::HostNotFound
                | ureq::Error::ConnectionFailed => {
                    Failure::transient("eric_download_failed", message)
                }
                _ => Failure::new(4, "eric_download_failed", message),
            }
        })?;
        let status = response.status();
        match status {
            StatusCode::OK => return Ok(response.into_body()),
            StatusCode::MOVED_PERMANENTLY
            | StatusCode::FOUND
            | StatusCode::SEE_OTHER
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::PERMANENT_REDIRECT => {
                if hop == MAX_REDIRECTS {
                    break;
                }
                let target = response
                    .headers()
                    .get(ureq::http::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|location| url.join(location).ok())
                    .filter(|target| {
                        target.origin() == url.origin()
                            && target.username().is_empty()
                            && target.password().is_none()
                    })
                    .ok_or_else(|| {
                        Failure::new(
                            4,
                            "eric_redirect_refused",
                            "ERiC download redirects must stay on https://download.elster.de",
                        )
                    })?;
                url = target;
            }
            _ => {
                let message = format!("ERiC download returned HTTP {status}");
                return Err(
                    if status.is_server_error()
                        || matches!(
                            status,
                            StatusCode::REQUEST_TIMEOUT | StatusCode::TOO_MANY_REQUESTS
                        )
                    {
                        Failure::transient("eric_download_failed", message)
                    } else {
                        Failure::new(4, "eric_download_failed", message)
                    },
                );
            }
        }
    }
    Err(Failure::new(
        4,
        "eric_redirect_refused",
        "Too many ERiC download redirects",
    ))
}

fn install_from(
    package: &Package<'_>,
    root: &Path,
    body: impl Read,
    log_dir: Option<&Path>,
    verbose: bool,
) -> Result<(PathBuf, Option<Vec<TraceEvent>>), Failure> {
    let versions = root.join(VERSIONS_DIR);
    fs::create_dir_all(&versions).map_err(|err| file_error(&versions, err))?;
    let mut archive = tempfile::tempfile_in(root).map_err(|err| file_error(root, err))?;
    verify_download(body, &mut archive, package.sha256, root)?;
    let staging = tempfile::Builder::new()
        .prefix(&format!("{}-", package.version))
        .tempdir_in(&versions)
        .map_err(|err| file_error(&versions, err))?;
    let dir = staging.path();
    let platform = format!("ERiC-{}/{PACKAGE_NAME}", package.version);
    extract_platform(archive, Path::new(&platform), dir)?;

    let marker = dir.join(TERMS_MARKER);
    File::create_new(&marker).map_err(|err| file_error(&marker, err))?;
    let mut probe = Probe::new(dir, verbose);
    probe.check(dir, log_dir)?;

    // Renaming a sibling link replaces the pointer in one step. Readers resolve it
    // once so an overlapping install cannot mix the two versions' libraries.
    let switch = tempfile::tempdir_in(root).map_err(|err| file_error(root, err))?;
    let pending = switch.path().join(ACTIVE_LINK);
    let relative = Path::new(VERSIONS_DIR).join(dir.file_name().expect("staging directory name"));
    symlink(relative, &pending).map_err(|err| file_error(&pending, err))?;
    let active = root.join(ACTIVE_LINK);
    // Do not replace an unrelated entry at the reserved pointer path.
    super::installed_dir(root)?;
    fs::rename(pending, &active).map_err(|err| file_error(&active, err))?;
    Ok((staging.keep(), probe.trace))
}

fn verify_download(
    mut source: impl Read,
    archive: &mut File,
    expected: &str,
    root: &Path,
) -> Result<(), Failure> {
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    let mut bytes = 0;
    loop {
        let n = match source.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(err) => {
                return Err(Failure::transient(
                    "eric_download_failed",
                    format!("Could not read ERiC download: {err}"),
                ));
            }
        };
        bytes += n;
        if bytes > MAX_DOWNLOAD {
            return Err(Failure::new(
                4,
                "eric_download_too_large",
                "ERiC download exceeds 1 GiB",
            ));
        }
        let chunk = &buffer[..n];
        hasher.update(chunk);
        archive
            .write_all(chunk)
            .map_err(|err| file_error(root, err))?;
    }
    let actual = format!("{:x}", hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(Failure::new(
            4,
            "eric_sha256_mismatch",
            format!("ERiC package SHA-256 mismatch: expected {expected}, received {actual}"),
        ));
    }
    archive.rewind().map_err(|err| file_error(root, err))
}

fn extract_platform(file: File, prefix: &Path, dir: &Path) -> Result<(), Failure> {
    let mut archive = ZipArchive::new(BufReader::new(file)).map_err(archive_error)?;
    for index in 0..archive.len() {
        let name = archive
            .name_for_index(index)
            .expect("ZIP entry index")
            .map_err(archive_error)?;
        let Ok(relative) = Path::new(name.as_ref()).strip_prefix(prefix) else {
            continue;
        };
        if relative.as_os_str().is_empty() {
            continue;
        }
        if name.contains('\0')
            || !relative
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
        {
            return Err(archive_error(format_args!("unsafe entry {name:?}")));
        }
        let path = dir.join(relative);
        let mut entry = archive.by_index(index).map_err(archive_error)?;
        if entry.is_symlink() {
            return Err(archive_error(format_args!(
                "symlink entry {:?} is not allowed",
                entry.name()
            )));
        }
        if entry.is_dir() {
            fs::create_dir_all(&path).map_err(|err| file_error(&path, err))?;
        } else {
            let parent = path.parent().expect("archive entry parent");
            fs::create_dir_all(parent).map_err(|err| file_error(parent, err))?;
            let mut output = File::create_new(&path).map_err(|err| file_error(&path, err))?;
            let bytes = io::copy(&mut entry, &mut output).map_err(|err| file_error(&path, err))?;
            if bytes != entry.size() {
                return Err(archive_error(format_args!(
                    "size mismatch for {:?}",
                    entry.name()
                )));
            }
            if let Some(mode) = entry.unix_mode() {
                output
                    .set_permissions(Permissions::from_mode(mode & 0o777))
                    .map_err(|err| file_error(&path, err))?;
            }
        }
    }
    if !dir.join(LICENSE).is_file() {
        return Err(archive_error(format_args!(
            "missing {LICENSE} under {}",
            prefix.display()
        )));
    }
    Ok(())
}

fn archive_error(err: impl std::fmt::Display) -> Failure {
    Failure::new(
        4,
        "eric_archive_invalid",
        format!("Invalid ERiC archive: {err}"),
    )
}

fn file_error(path: &Path, err: io::Error) -> Failure {
    Failure::new(
        4,
        "eric_install_failed",
        format!("Could not install ERiC at '{}': {err}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use zip::{ZipWriter, write::SimpleFileOptions};

    const SHAPE: &str = "ERiC-44.3.6.0/Linux-x86_64";

    fn add(zip: &mut ZipWriter<Cursor<Vec<u8>>>, name: &str, data: &[u8]) {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(data).unwrap();
    }

    fn result_code<T>(result: Result<T, Failure>) -> String {
        let Err(error) = result else {
            panic!("expected failed installation")
        };
        serde_json::to_value(error).unwrap()["code"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    #[test]
    fn bad_integrity_or_test_load_leaves_the_active_version_intact() {
        let root = tempfile::tempdir().unwrap();
        let previous = root.path().join("versions/44.1.0.0");
        fs::create_dir_all(&previous).unwrap();
        symlink("versions/44.1.0.0", root.path().join(ACTIVE_LINK)).unwrap();
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        add(&mut zip, &format!("{SHAPE}/{LICENSE}"), b"invented terms");
        add(
            &mut zip,
            &format!("{SHAPE}/lib/libericapi.so"),
            b"not a library",
        );
        let bytes = zip.finish().unwrap().into_inner();
        let digest = format!("{:x}", Sha256::digest(&bytes));
        for (sha256, code) in [
            (DEFAULT_SHA256, "eric_sha256_mismatch"),
            (&digest, "eric_load_failed"),
        ] {
            let package = Package {
                version: DEFAULT_VERSION,
                major: 44,
                sha256,
            };
            assert_eq!(
                result_code(install_from(&package, root.path(), &*bytes, None, false)),
                code
            );
            assert_eq!(
                super::super::installed_dir(root.path()).ok().flatten(),
                Some(previous.clone())
            );
            assert_eq!(
                fs::read_dir(root.path().join(VERSIONS_DIR))
                    .unwrap()
                    .count(),
                1
            );
        }
    }

    #[test]
    #[ignore = "requires the shared libraries from an unpacked official ERiC 44.3.6.0 package"]
    fn extracts_only_linux_and_activates_after_a_real_test_load() {
        let source = PathBuf::from(
            std::env::var_os("KIECLI_TEST_ERIC_HOME").expect("unpacked ERiC directory"),
        );
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        add(
            &mut zip,
            "ERiC-44.3.6.0/Windows-x86_64/ignore.txt",
            b"other platform",
        );
        add(
            &mut zip,
            &format!("{SHAPE}/{LICENSE}"),
            &fs::read(source.join(LICENSE)).unwrap(),
        );
        for library in [
            "libericapi.so",
            "libericxerces.so",
            "libeSigner.so",
            "libotto.so",
            "plugins/libcommonData.so",
            "plugins/libcheckElsterDatenabholung.so",
        ] {
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            zip.start_file(format!("{SHAPE}/lib/{library}"), options)
                .unwrap();
            io::copy(
                &mut File::open(source.join("lib").join(library)).unwrap(),
                &mut zip,
            )
            .unwrap();
        }
        let bytes = zip.finish().unwrap().into_inner();
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let package = Package {
            version: DEFAULT_VERSION,
            major: 44,
            sha256: &digest,
        };
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("versions/44.1.0.0")).unwrap();
        symlink("versions/44.1.0.0", root.path().join(ACTIVE_LINK)).unwrap();
        let (dir, _) = install_from(&package, root.path(), &*bytes, None, false)
            .unwrap_or_else(|error| panic!("{}", serde_json::to_string(&error).unwrap()));
        assert_eq!(
            super::super::installed_dir(root.path())
                .ok()
                .flatten()
                .as_deref(),
            Some(dir.as_path())
        );
        assert!(!dir.join("ignore.txt").exists());
    }
}
