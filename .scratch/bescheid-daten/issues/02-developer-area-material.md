# Download developer-area Bescheid material

Type: task
Status: resolved
Blocked by:

## Question

Some format documents for Bescheiddaten are only in the logged-in ELSTER developer area, which agents cannot reach. HITL: log in and download them into `docs/API/` (gitignored, never committed), then record here the file names and versions, and any licence or redistribution note shown with them.

Named by [Bescheiddaten formats per Bereitstellungsdatenart](01-bescheiddaten-formats.md):

1. `Bescheidnr_*.zip` (`Bescheidwertnummern_ESt.xml`, `_GSt.xml`, `_USt.xml`; ERiC-Entwicklerhandbuch ch. 9.1 and Table 4-5). The only definition of what each Bescheidwertnummer means; essential.
2. Schlüsselkatalog "Art des Schreibens" für DIVA (Table 4-5), behind the DIVA `dokumenttyp` metadata. Useful.
3. Skip unless asked: Dokumentation zum Bescheid zur InvStG51Feststellung (PDF-only kind) and the Musterbescheide Grundsteuer (sample PDFs, no data format).

Record especially any licence or redistribution wording on item 1: it decides [Ship the Bescheidwertnummer table?](06-ship-number-table.md).

## Answer

**Downloaded on 2026-11-19 into `docs/API/`: `Bescheidnr_Juni_2026.zip`, plus the InvStG51 XML-Anhang documentation and its sample; no licence or redistribution wording anywhere in them.** The Schlüsselkatalog "Art des Schreibens" was not among the Bescheid downloads.

- `Bescheidnr_Juni_2026.zip` (entries dated 2026-08-11) holds `Bescheidwertnummern_ESt.ods`, `_USt.ods` and `_GSt.ods`. They are OpenDocument spreadsheets, not the XML files the handbook names. Sheet `Bescheidwertnr` has the columns Bescheidwertnr. (10 digits), Bezeichnung, Dezimalstellen and Art (F Festsetzung / A Abrechnung / V Vorauszahlung): 1,865 ESt, 401 GSt and 229 USt numbers. Sheet `Fußnoten_und_Erläuterungen` explains value codes and formats.
- Correction to [Bescheiddaten formats per Bereitstellungsdatenart](01-bescheiddaten-formats.md): Vorbehalt and Vorläufigkeit *are* in BDS-2. They are not in the header but in a `W` record: Bescheidwertnummer `0680000853` "Bescheidkennzeichnung" in ESt and GSt (0 erstmalig endgültig, 1 Vorbehalt der Nachprüfung, 2 vorläufig, 4 Berichtigung, and their combinations, the same 0–7 scale as the DIVA2 `vorlaeufigkeitsmerker`).
- Footnotes matter for parsing: NRW sends GSt amounts without cents and currently no GSt Vorauszahlungen; dates come as `TMMJJJJ`/`TTMMJJJJ` or `TTMMJJ`; some numbers carry free or standardised explanation texts.
- `Dokumentation_XML-Anhang_zu_InvStG51Feststellung_v1.pdf` (version 1.0.0, 18.12.2024) and `MUSTER_Anlage_zur_Ges_einh_Fest_51_InvStG.xml`: the § 51 InvStG Feststellung is sent "standardmäßig" with an XML **Anhang** next to the PDF (namespace `…/invstg51feststellung/v3`, not binding). This corrects ticket 01's "every DIVA kind is PDF plus metadata only": a DIVA Bescheid can carry Bescheiddaten as an XML Anhang.
- Licence: neither the zip, the spreadsheets nor the PDF contain any licence, copyright or redistribution text. The PDF names its source as "Entwicklerbereich bei www.elster.de".
