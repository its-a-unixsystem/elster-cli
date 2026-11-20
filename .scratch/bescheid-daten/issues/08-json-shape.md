# JSON shape of Bescheiddaten

Type: prototype
Status: open
Blocked by:

## Question

What does the Bescheiddaten JSON look like? Hand-write invented JSON (no real values) for three cases and react to it:

1. one BDS-2 ESB with `V`/`D`/`W` records and the three Teilbescheide Festsetzung, Abrechnung and Vorauszahlung (layout: ERiC-Entwicklerhandbuch 15.4, see [Bescheiddaten formats per Bereitstellungsdatenart](01-bescheiddaten-formats.md));
2. one XUnternehmen.Gewerbesteuer `bescheide.gewerbesteuer.0001`, carrying the provenance marking that it is not the Bescheid and was not compared with the PDF ([XUnternehmen.Gewerbesteuer in the Gewerbesteuerbescheid](05-xunternehmen-gewerbesteuer.md));
3. one decoded DIVA2 metadata block beside v1's raw `meta`.

Settle: how faithful-copy records and named key figures sit side by side; one Bescheidkennzeichnung shape shared by all three sources; typing of amounts (v1 precedent: decimal strings), dates (BDS-2 `TMMJJJJ`/`TTMMJJJJ`/`TTMMJJ`), cent-less NRW GSt amounts and codes; how the format version (BDS-2 `0003`/`0004`, XUnternehmen namespace) is exposed. Which Bescheidwertnummern get named fields stays a slot until [Ship the Bescheidwertnummer table?](06-ship-number-table.md) resolves.
