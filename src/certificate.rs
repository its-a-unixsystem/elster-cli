// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::io::{self, Write};
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de};
use time::{Date, Duration, Month, OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    Cli, Format,
    config::{self, Warning},
    eric::{self, CertificateReader, Probe, TraceEvent},
    error::Failure,
    pin::Pin,
    write_json,
};

#[derive(Deserialize)]
struct Properties {
    #[serde(rename = "Verschluesselungszertifikateigenschaften")]
    encryption: Option<Block>,
    #[serde(rename = "Signaturzertifikateigenschaften")]
    signature: Option<Block>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct Block {
    #[serde(
        rename(deserialize = "AusgestelltAm"),
        deserialize_with = "eric_time",
        serialize_with = "time::serde::rfc3339::serialize"
    )]
    #[schemars(with = "String", extend("format" = "date-time"))]
    gueltig_ab: OffsetDateTime,
    #[serde(
        rename(deserialize = "GueltigBis"),
        deserialize_with = "eric_time",
        serialize_with = "time::serde::rfc3339::serialize"
    )]
    #[schemars(with = "String", extend("format" = "date-time"))]
    gueltig_bis: OffsetDateTime,
    #[serde(rename(deserialize = "Signaturalgorithmus"))]
    signaturalgorithmus: Option<String>,
    #[serde(rename(deserialize = "PublicKeyBitLength"))]
    schluessellaenge: Option<u32>,
    #[serde(rename(deserialize = "PublicKeySHA1"))]
    public_key_sha1: Option<String>,
    #[serde(rename(deserialize = "Issuer"), default)]
    issuer: Names,
    #[serde(rename(deserialize = "Subjekt"), default)]
    subjekt: Names,
    #[serde(rename(deserialize = "Identifikationsmerkmaltyp"))]
    identifikationsmerkmaltyp: Option<String>,
    #[serde(rename(deserialize = "Verifikationsart"))]
    verifikationsart: Option<String>,
    #[serde(rename = "Registrierertyp", skip_serializing)]
    registrierertyp: Option<String>,
    #[serde(rename = "TokenTyp", skip_serializing)]
    token_typ: Option<String>,
    #[serde(rename = "Testzertifikat", skip_serializing)]
    testzertifikat: Option<bool>,
}

#[derive(Deserialize, Default, JsonSchema)]
#[schemars(with = "Vec<Name>")]
struct Names {
    #[serde(rename = "Info", default)]
    entries: Vec<Name>,
}

impl Serialize for Names {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.entries.serialize(serializer)
    }
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct Name {
    #[serde(rename(deserialize = "Name"))]
    name: String,
    #[serde(rename(deserialize = "Wert"))]
    value: String,
}

fn eric_time<'de, D: Deserializer<'de>>(deserializer: D) -> Result<OffsetDateTime, D::Error> {
    let raw = <&str>::deserialize(deserializer)?;
    let invalid = || de::Error::custom("invalid ERiC Certificate timestamp");
    let Some(digits) = raw.strip_suffix('Z') else {
        return Err(invalid());
    };
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid());
    }
    let (year, rest) = match digits.len() {
        14 => (&digits[..4], &digits[4..]),
        12 => (&digits[..2], &digits[2..]),
        _ => return Err(invalid()),
    };
    let mut value = year.parse::<i32>().map_err(|_| invalid())?;
    if year.len() == 2 {
        value += if value < 50 { 2000 } else { 1900 };
    }
    let number = |start| rest[start..start + 2].parse().map_err(|_| invalid());
    let month = Month::try_from(number(0)?).map_err(|_| invalid())?;
    let day = number(2)?;
    let (hour, minute, second) = (number(4)?, number(6)?, number(8)?);
    Date::from_calendar_date(value, month, day)
        .and_then(|date| date.with_hms(hour, minute, second))
        .map(|time| time.assume_utc())
        .map_err(|_| invalid())
}

#[derive(Serialize, JsonSchema)]
pub(crate) struct Status<'a> {
    #[serde(with = "time::serde::rfc3339")]
    #[schemars(with = "String", extend("format" = "date-time"))]
    gueltig_ab: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schemars(with = "String", extend("format" = "date-time"))]
    gueltig_bis: OffsetDateTime,
    tage_verbleibend: i64,
    abgelaufen: bool,
    testzertifikat: bool,
    registrierertyp: Option<&'a str>,
    organisationszertifikat: bool,
    token_typ: Option<&'a str>,
    konto_id: Option<&'a str>,
    verschluesselungszertifikat: Option<&'a Block>,
    signaturzertifikat: Option<&'a Block>,
    test: bool,
    profile: Option<String>,
    warnings: Vec<Warning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    trace: Option<Vec<TraceEvent>>,
}

impl Properties {
    fn read(
        reader: &impl CertificateReader,
        path: &Path,
        pin: &Pin,
        trace: &mut Option<Vec<TraceEvent>>,
    ) -> Result<Self, Failure> {
        reader.inspect_certificate(path, pin, trace, Self::from_xml)
    }

    fn from_xml(xml: &str) -> Result<Self, Failure> {
        quick_xml::de::from_str(xml).map_err(|_| invalid_properties())
    }

    fn report(
        &self,
        now: OffsetDateTime,
        settings: config::Settings,
        trace: Option<Vec<TraceEvent>>,
    ) -> Result<Status<'_>, Failure> {
        let (signature, encryption) = (self.signature.as_ref(), self.encryption.as_ref());
        let blocks = || signature.into_iter().chain(encryption);
        let gueltig_ab = blocks()
            .map(|block| block.gueltig_ab)
            .min()
            .ok_or_else(invalid_properties)?;
        let gueltig_bis = blocks()
            .map(|block| block.gueltig_bis)
            .min()
            .ok_or_else(invalid_properties)?;
        let remaining = gueltig_bis - now;
        let abgelaufen = now > gueltig_bis || blocks().any(|block| now < block.gueltig_ab);
        let registrierertyp = blocks().find_map(|block| block.registrierertyp.as_deref());
        let token_typ = blocks().find_map(|block| block.token_typ.as_deref());
        let konto_id = if token_typ == Some("Software") {
            blocks()
                .flat_map(|block| &block.subjekt.entries)
                .find(|entry| entry.name == "CN")
                .map(|entry| entry.value.as_str())
        } else {
            None
        };
        let mut warnings = settings.warnings;
        if abgelaufen {
            warnings.push(Warning::new(
                "certificate_expired",
                "The Certificate is expired or not yet valid",
            ));
        }
        if now <= gueltig_bis && remaining < Duration::days(60) {
            warnings.push(Warning::new(
                "certificate_expires_soon",
                "The Certificate expires in less than 60 days",
            ));
        }
        Ok(Status {
            gueltig_ab,
            gueltig_bis,
            tage_verbleibend: remaining.whole_days(),
            abgelaufen,
            testzertifikat: blocks().any(|block| block.testzertifikat == Some(true)),
            registrierertyp,
            organisationszertifikat: registrierertyp == Some("Organisation"),
            token_typ,
            konto_id,
            verschluesselungszertifikat: encryption,
            signaturzertifikat: signature,
            test: settings.test,
            profile: settings.profile,
            warnings,
            trace,
        })
    }
}

pub(crate) fn check_for_request(
    xml: &str,
    now: OffsetDateTime,
    warnings: &mut Vec<Warning>,
) -> Result<bool, Failure> {
    let properties = Properties::from_xml(xml)?;
    let blocks = || {
        properties
            .signature
            .iter()
            .chain(properties.encryption.iter())
    };
    let expiry = blocks()
        .map(|block| block.gueltig_bis)
        .min()
        .ok_or_else(invalid_properties)?;
    if now > expiry {
        return Err(
            Failure::new(3, "certificate_expired", "The Certificate is expired")
                .help("Renew the Certificate in Mein ELSTER."),
        );
    }
    if blocks().any(|block| now < block.gueltig_ab) {
        return Err(Failure::new(
            3,
            "certificate_not_yet_valid",
            "The Certificate is not yet valid",
        ));
    }
    if expiry - now < Duration::days(60) {
        warnings.push(Warning::new(
            "certificate_expires_soon",
            "The Certificate expires in less than 60 days",
        ));
    }
    Ok(blocks().any(|block| block.testzertifikat == Some(true)))
}

impl Status<'_> {
    fn write_text(&self, mut out: impl Write) -> io::Result<()> {
        write!(out, "Certificate validity: ")?;
        self.gueltig_ab
            .format_into(&mut out, &Rfc3339)
            .map_err(io::Error::other)?;
        write!(out, " to ")?;
        self.gueltig_bis
            .format_into(&mut out, &Rfc3339)
            .map_err(io::Error::other)?;
        writeln!(out, " ({} days remaining)", self.tage_verbleibend)?;
        if let Some(id) = self.konto_id {
            writeln!(out, "Konto-ID: {}", id.escape_debug())?;
        }
        if let Some(kind) = self.registrierertyp {
            writeln!(out, "Registrant type: {}", kind.escape_debug())?;
        }
        writeln!(out, "Test Certificate: {}", self.testzertifikat)?;
        for warning in &self.warnings {
            warning.write_text(&mut out)?;
        }
        Ok(())
    }
}

fn invalid_properties() -> Failure {
    Failure::new(
        1,
        "certificate_properties_invalid",
        "ERiC returned invalid Certificate properties",
    )
    .note(eric::LOG_NOTE)
}

pub(crate) fn status(cli: &Cli, mut out: impl Write) -> Result<(), Failure> {
    let (settings, path, pin) = config::certificate_settings(cli)?;
    let installed = eric::installed_dir(&settings.eric_dir)?;
    let dir = installed.as_deref().unwrap_or(&settings.eric_dir);
    let mut probe = Probe::new(dir, cli.verbose);
    let properties = probe.with_session(dir, cli.eric_log.as_deref(), |session, trace| {
        Properties::read(session, &path, &pin, trace)
    })?;
    drop(pin);
    let report = properties.report(OffsetDateTime::now_utc(), settings, probe.trace)?;
    match cli.format {
        Format::Json => write_json(out, &report),
        Format::Text => report.write_text(&mut out),
    }
    .map_err(|err| Failure::new(1, "output_failed", err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeEric(&'static str);

    impl CertificateReader for FakeEric {
        fn inspect_certificate<T>(
            &self,
            _: &Path,
            _: &Pin,
            _: &mut Option<Vec<TraceEvent>>,
            read: impl FnOnce(&str) -> Result<T, Failure>,
        ) -> Result<T, Failure> {
            read(self.0)
        }
    }

    #[test]
    fn reports_the_shared_validity_window_and_warns_at_its_edges() {
        let eric = FakeEric(
            "<EricHoleZertifikatEigenschaften>\
              <Signaturzertifikateigenschaften>\
                <AusgestelltAm>500101000000Z</AusgestelltAm>\
                <GueltigBis>20500101000000Z</GueltigBis>\
              </Signaturzertifikateigenschaften>\
              <Verschluesselungszertifikateigenschaften>\
                <AusgestelltAm>490101000000Z</AusgestelltAm>\
                <GueltigBis>491231000000Z</GueltigBis>\
              </Verschluesselungszertifikateigenschaften>\
            </EricHoleZertifikatEigenschaften>",
        );
        let pin = crate::pin::Source::Env(zeroize::Zeroizing::new(b"dummy".to_vec()))
            .read(&mut Vec::new())
            .ok()
            .unwrap();
        let properties = Properties::read(&eric, Path::new("unused.pfx"), &pin, &mut None)
            .ok()
            .unwrap();
        let report = |now: &str| {
            let settings = config::Settings {
                eric_dir: Default::default(),
                profile: None,
                test: false,
                warnings: Vec::new(),
            };
            serde_json::to_value(
                properties
                    .report(
                        OffsetDateTime::parse(now, &Rfc3339).unwrap(),
                        settings,
                        None,
                    )
                    .ok()
                    .unwrap(),
            )
            .unwrap()
        };
        let sixty_days = report("2049-11-01T00:00:00Z");
        assert_eq!(sixty_days["gueltig_ab"], "1950-01-01T00:00:00Z");
        assert_eq!(sixty_days["gueltig_bis"], "2049-12-31T00:00:00Z");
        assert_eq!(sixty_days["tage_verbleibend"], 60);
        assert_eq!(
            sixty_days["signaturzertifikat"]["gueltig_ab"],
            "1950-01-01T00:00:00Z"
        );
        assert_eq!(sixty_days["warnings"], serde_json::json!([]));
        let soon = report("2049-11-01T00:00:01Z");
        assert_eq!(soon["warnings"][0]["code"], "certificate_expires_soon");
        for now in ["2048-12-31T00:00:00Z", "2050-01-01T00:00:00Z"] {
            let outside = report(now);
            assert_eq!(outside["abgelaufen"], true);
            assert_eq!(outside["warnings"][0]["code"], "certificate_expired");
        }
    }
}
