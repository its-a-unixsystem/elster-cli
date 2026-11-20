# Postfach lifecycle and attachments

Type: research
Status: resolved
Blocked by:

## Question

How does a Postfach-Nachricht move through ELSTER Datenabholung, and what can a client observe at each step?

Cover: what PostfachStatus, PostfachAnfrage and PostfachBestaetigung each return; filters (veranlagungszeitraum, ag-steuernummer, lieferticket, …) and pagination; whether a Postfach-Nachricht can still be retrieved after Bestätigung, and for how long (DIVA-Löschzeitraum); which Bereitstellungsdatenarten an organisation (Organisationszertifikat) can receive versus a natural person; the shape of returned content (PDF, structured Bescheiddaten, formatted HTML); the Statusabfrage; and the full attachment flow via the Objektspeicher (`objektid`, signatures over it, POZ vs CEZ certificates, the `x-elster-*` headers in the OTTER OpenAPI).

Output: a lifecycle description and the list of operations a v1 CLI must expose to cover it.

## Answer

A Postfach-Nachricht moves through three steps: Anfrage (`PostfachStatus` for counts, `PostfachAnfrage` for metadata and Objektspeicher object IDs, paged by `paginierungLetzteBestId`, up to 10,000 per page), Abholung (OTTER `fetch` per object ID, signed with the Certificate, no separate certificate or registration documented), then Bestätigung (`PostfachBestaetigung`, up to 100 IDs per block, mandatory within 24 h of a successful Abholung). `Statusabfrage` reports 05..99 states including `30-BESTAETIGT` and `90-GELOESCHT`.
- Retention after Bestätigung is not documented. Each record carries `verfuegbarbis` (its deletion date). The public ELSTER FAQ says Bescheiddaten are deleted 60 days after Abholung (180 days if never collected). `einschraenkung="alle"` suggests confirmed records stay listed; verify on the test system. The DIVA-Löschzeitraum is not given as a number.
- `PostfachAnfrage` must name every Bereitstellungsdatenart (no wildcard); filters: `lieferticket`, `ag-steuernummer`, `mandantennummer`, and `veranlagungszeitraum` (announced). Which Datenarten an Organisationszertifikat vs natural person receives is undocumented; `PostfachStatus` is the discovery call.
- Content: XML metadata plus object references; the main payload is text/XML (ISO-8859-15 or UTF-8), PDFs arrive as `Anhaenge`; objects are delivered as CMS/PKCS#7 EnvelopedData for the Certificate.
- OTTER needs a TLS client certificate that the docs never identify (open; try the public `extern-offen.p12` in the spike).

Full findings, operations list and open points: [research/02-postfach-lifecycle.md](../research/02-postfach-lifecycle.md)
