# ERiC and the Kontoabfrage

Type: research
Status: resolved
Blocked by:

## Question

Does the Kontoabfrage require ERiC, and if so, what would depending on ERiC cost an open-source Go or Rust CLI?

Cover: whether ElsterKontoabfrage is truly excluded from the Offene Schnittstelle (it is absent from `Verzeichnis_der_Datenarten.csv`) or has any sanctioned open route; what ERiC does for it (plausibility checks, encryption, signing); the ERiC licence terms (redistribution, use from open-source software, whether each user must download it with their own developer login); supported platforms and architectures (Linux x86_64/arm64, macOS arm64/x86_64); the C API surface needed for a Kontoabfrage and how it is called from Go (cgo) and Rust (FFI); update cadence (annual ERiC versions, end-of-life of old versions).

## Answer

Yes, in practice: the Kontoabfrage needs ERiC, but only by policy. It is absent from `Verzeichnis_der_Datenarten.csv` (all 18 listed rows are Datenabholung/Postfach, ElsterLohn, ElsterLohn2 and ElsterKMV), and the Kontoabfrage Schnittstellenbeschreibung names no open route. Its wire format (`send-Auth`, same ElsterXML) is the same as the open procedures, so a self-built client is technically plausible but unsanctioned and untested.

- ERiC licence (release-41 text, secondary copy): free, gated by manufacturer registration. Redistribution only inside the manufacturer's own product, no sublicensing. A public CLI release cannot bundle ERiC.
- Existing open-source bindings (erica, eric-rs, haben) all ship without ERiC and have the user supply it. Linux x86_64 is verified. macOS arm64 is only suggested. Linux arm64 and macOS x86_64 are unverified.
- Rust has `eric-bindings`/`eric-sdk`. No Go binding found. ERiC 44 changed the `EricBearbeiteVorgang` signature, so version handling is required.
- ERiC majors come yearly, with a November release. No official EOL table was found.
- The Kontoabfrage response is moving to BIENE (`kontoabfrage_antwort`), finanzamtsweise since November 2025.
- Recommendation: keep v1 ERiC-free and move the Kontoabfrage to v2 with a seam for an optional user-supplied ERiC. Ask the BayLfSt about a non-ERiC route.

Full findings: [research/03-eric-kontoabfrage.md](../research/03-eric-kontoabfrage.md)
