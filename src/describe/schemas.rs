// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use schemars::{Schema, SchemaGenerator, generate::SchemaSettings};

use crate::{certificate, config, eric, error::Report, kontoabfrage, postfach};

fn generator() -> SchemaGenerator {
    // With the serialize contract, `#[schemars(required)]` on a skipped Option
    // means non-null when present, without making the property mandatory.
    SchemaSettings::draft2020_12()
        .for_serialize()
        .into_generator()
}

pub(super) fn stdout(path: &str) -> Option<Schema> {
    let generator = generator();
    Some(match path {
        "describe" => generator.into_root_schema_for::<super::Description<'_>>(),
        "config show" => generator.into_root_schema_for::<config::Show<'_>>(),
        "eric install" => generator.into_root_schema_for::<eric::install::Installed<'_>>(),
        "eric status" => generator.into_root_schema_for::<crate::Status<'_>>(),
        "zertifikat status" => generator.into_root_schema_for::<certificate::Status<'_>>(),
        "kontoabfrage offene-betraege" | "kontoabfrage zahlungen" | "kontoabfrage sollstellungen" => {
            generator.into_root_schema_for::<kontoabfrage::Output>()
        }
        _ => return postfach::stdout_schema(path.strip_prefix("postfach ")?, generator),
    })
}

pub(super) fn stderr() -> Schema {
    generator().into_root_schema_for::<Report<'_>>()
}
