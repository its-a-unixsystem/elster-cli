// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use super::*;
use time::macros::date;

fn validate_output(command: &str, art: &'static str, reply: Reply) -> serde_json::Value {
    crate::describe::contract::stdout(
        command,
        &super::super::Output {
            steuernummer: "9199026211407".to_owned(),
            bundesfinanzamtsnummer: "9199".to_owned(),
            abfrageart: art,
            teile: reply,
            test: true,
            profile: None,
            warnings: Vec::new(),
            trace: None,
        },
    )
}

fn one_open() -> Vec<Input> {
    vec![Input::Open]
}

#[test]
fn maps_a_legacy_response_and_normalizes_amounts() {
    let xml = r#"<Elster><DatenTeil><Nutzdatenblock><Nutzdaten><kontoabfrage version="6"><kontoabfrage-teil><kontoabfrage-input><kontoabfrage-art>O</kontoabfrage-art><steuernummer>9199026211407</steuernummer></kontoabfrage-input><kontoabfrage-output><abfrage-tagesdatum>08.05.2023 15:19:34</abfrage-tagesdatum><abfrage-steuerart><steuerart-klartext>Einkommensteuer</steuerart-klartext><steuerart-teilbetrag><teilbetrag-zeitraum>2020</teilbetrag-zeitraum><teilbetrag-faelligkeit>26.04.2021</teilbetrag-faelligkeit><teilbetrag-wert>-2.061,00</teilbetrag-wert></steuerart-teilbetrag></abfrage-steuerart><abfrage-gesamtsumme>-2.061,00</abfrage-gesamtsumme></kontoabfrage-output><bearbeitungshinweis><return-code>0</return-code><return-text>OK</return-text></bearbeitungshinweis></kontoabfrage-teil></kontoabfrage></Nutzdaten></Nutzdatenblock></DatenTeil></Elster>"#;
    let reply = match read(0, xml, None, "9199026211407", &one_open(), true) {
        Ok(reply) => reply,
        Err(err) => panic!(
            "unexpected response error: {}",
            serde_json::to_value(err).unwrap()
        ),
    };
    let value = validate_output("kontoabfrage offene-betraege", "O", reply);
    assert_eq!(value["teile"][0]["antwort_format"], "alt");
    assert_eq!(
        value["teile"][0]["steuerarten"][0]["teilbetraege"][0]["wert"],
        "-2061.00"
    );
    assert!(value["xml"].as_str().unwrap().starts_with("<kontoabfrage"));
}

#[test]
fn matches_reordered_payment_answers_by_echoed_input() {
    let inputs = vec![
        Input::Payment {
            steuerart: super::super::Steuerart::Est,
            ab: date!(2026 - 01 - 02),
            ein_jahr: false,
        },
        Input::Payment {
            steuerart: super::super::Steuerart::Ust,
            ab: date!(2026 - 01 - 02),
            ein_jahr: true,
        },
    ];
    let xml = r#"<Elster><DatenTeil><Nutzdatenblock><Nutzdaten><kontoabfrage version="6"><kontoabfrage-teil><kontoabfrage-input><kontoabfrage-art>I</kontoabfrage-art><steuernummer>9199026211407</steuernummer><steuerart>USt</steuerart><wertstellungsdatum option="J">02012026</wertstellungsdatum></kontoabfrage-input><kontoabfrage_antwort><stpfl><finanzamt>9199</finanzamt><abfragedatum>2026-01-03T12:00:00</abfragedatum></stpfl><steuerkontoTilgungen/></kontoabfrage_antwort><bearbeitungshinweis><return-code>0</return-code></bearbeitungshinweis></kontoabfrage-teil><kontoabfrage-teil><kontoabfrage-input><kontoabfrage-art>I</kontoabfrage-art><steuernummer>9199026211407</steuernummer><steuerart>ESt</steuerart><wertstellungsdatum option="V">02012026</wertstellungsdatum></kontoabfrage-input><kontoabfrage_antwort><stpfl/><steuerkontoTilgungen><zeile><text>Payment</text><zufluss>1.234,50</zufluss></zeile></steuerkontoTilgungen></kontoabfrage_antwort><bearbeitungshinweis><return-code>0</return-code></bearbeitungshinweis></kontoabfrage-teil></kontoabfrage></Nutzdaten></Nutzdatenblock></DatenTeil></Elster>"#;
    let reply = match read(0, xml, None, "9199026211407", &inputs, false) {
        Ok(reply) => reply,
        Err(err) => panic!(
            "unexpected response error: {}",
            serde_json::to_value(err).unwrap()
        ),
    };
    let value = validate_output("kontoabfrage zahlungen", "I", reply);
    assert_eq!(value["teile"][0]["steuerart"], "USt");
    assert_eq!(value["teile"][1]["steuerart"], "ESt");
    assert_eq!(value["teile"][1]["zeilen"][0]["zufluss"], "1234.50");
}

#[test]
fn maps_header_only_authorization_rejection() {
    let xml = r#"<Elster><TransferHeader><RC><Rueckgabe><Code>230077012</Code><Text>Not authorized</Text></Rueckgabe></RC></TransferHeader></Elster>"#;
    let failure = match read(
        610101292,
        xml,
        Some("rejected".into()),
        "9199026211407",
        &one_open(),
        false,
    ) {
        Ok(_) => panic!("expected an authorization failure"),
        Err(failure) => failure,
    };
    crate::describe::contract::stderr(&failure);
    let json = serde_json::to_value(failure).unwrap();
    assert_eq!(json["code"], "kontoabfrage_authorization_required");
    assert_eq!(json["cause"][0]["code"], 230077012);
}

#[test]
fn retries_backend_failures_and_keeps_codes_without_result_text() {
    for code in [950017008, 950019011] {
        let xml = format!(
            "<Elster><TransferHeader><RC><Rueckgabe><Code>{code}</Code></Rueckgabe></RC></TransferHeader></Elster>"
        );
        let failure = match read(610101292, &xml, None, "9199026211407", &one_open(), false) {
            Ok(_) => panic!("expected a retryable backend failure"),
            Err(failure) => failure,
        };
        assert_eq!(failure.exit, 6);
        crate::describe::contract::stderr(&failure);
        let json = serde_json::to_value(failure).unwrap();
        assert_eq!(json["retry"], true);
        assert_eq!(json["cause"][0]["code"], code);
    }
}
