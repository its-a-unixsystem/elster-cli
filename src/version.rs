// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use serde::Deserialize;

#[derive(Deserialize)]
struct VersionReport<'a> {
    #[serde(rename = "Bibliothek", default, borrow)]
    libraries: Vec<LibraryVersion<'a>>,
}

#[derive(Deserialize)]
struct LibraryVersion<'a> {
    #[serde(rename = "Name")]
    name: &'a str,
    #[serde(rename = "Produktversion")]
    product_version: &'a str,
}

pub(crate) fn read(xml: &str, library: &str) -> Option<[u16; 4]> {
    let report: VersionReport<'_> = quick_xml::de::from_str(xml).ok()?;
    let version = report
        .libraries
        .into_iter()
        .find(|item| item.name == library)?;
    let mut parts = version.product_version.split(',');
    let mut numbers = [0; 4];
    for number in &mut numbers {
        *number = parts.next()?.trim().parse().ok()?;
    }
    parts.next().is_none().then_some(numbers)
}

#[cfg(test)]
mod tests {
    #[test]
    fn selects_the_main_library_product_version() {
        let xml = include_str!("../tests/fixtures/eric/version.xml");
        assert_eq!(super::read(xml, "libericapi.so"), Some([44, 17, 9, 3]));
    }
}
