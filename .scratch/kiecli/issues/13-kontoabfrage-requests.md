# Kontoabfrage requests and responses

Type: grilling
Status: resolved
Blocked by: 07

## Question

Which Kontoabfrage request kinds does v1 support (`O` offene Beträge, `I` geleistete Zahlungen with `steuerart` and `wertstellungsdatum`, `ZS` zum Soll gestellte Beträge with `steuerart` and `zeitraum`), what the caller must supply for each (Steuernummer, Bundesfinanzamtsnummer, filters), and how v1 returns both response shapes while Finanzämter migrate to BIENE (`kontoabfrage_antwort` vs `kontoabfrage-output`): one normalised JSON shape, raw XML, or both?

Context: [ERiC and the Kontoabfrage](03-eric-kontoabfrage.md) (schemas `eko_*_000006.xsd`, one Steuerfall per request, Steuernummern must match the Empfänger Finanzamt, a Vollmacht is presupposed); [ERiC for the Kontoabfrage](07-eric-decision.md) (sent through ERiC).

Where the identifiers come from is settled by [Configuration and secrets](10-config-and-secrets.md): per-profile config keys, overridable by flag and `KIECLI_*` env var; this ticket names the fields.

## Answer

**All three Abfragearten, one subcommand each, a faithful JSON copy of whichever response shape the Finanzamt returns.** Terms Abfrageart, Sollstellung, Steuerart, BIENE Steuerkontoauszug, Steuernummer and Bundesfinanzamtsnummer added to `GLOSSARY.md`. No ADR: nothing here is hard to reverse.

Facts from `docs/API/ElsterKontoabfrage_v2.1.3.zip`: one request holds up to 75 `kontoabfrage-teil`, which may mix Abfragearten; every Steuernummer in it must belong to the Empfänger Finanzamt; answers may come back in a different order than asked; Testmerker for the Kontoabfrage is `230000001` (not the Postfach's `700000001`); the Bundesfinanzamtsnummer is the first four digits of the ELSTER-format Steuernummer.

### Commands

```
kiecli kontoabfrage offene-betraege
kiecli kontoabfrage zahlungen      --ab D [--ein-jahr] [--steuerart X…]
kiecli kontoabfrage sollstellungen --steuerart X… --jahr YYYY…
```

1. All three Abfragearten (`O`, `I`, `ZS`) in v1, one per call. Mixing Abfragearten in one request is not offered.
2. `zahlungen`: `--ab` (ISO date, sent as `TTMMJJJJ`) is required; mode `V` (until today) by default, `--ein-jahr` sends `J`; `--steuerart` defaults to `alle`.
3. `sollstellungen`: `--steuerart` and `--jahr` both required, no `alle`. Repeated values fan out into the cartesian product of `kontoabfrage-teil` in one request; more than 75 is exit 2.
4. Steuerart is matched case-insensitively and sent in ELSTER spelling (`ESt`, `KSt`, `USt`, `LSt`, `GewSt`, `ZaSt`, `KapESt`); GewSt outside the Stadtstaaten is left to ELSTER to reject.

### Taxpayer identifiers

5. Settings `steuernummer` and optional `bundesland` (two-letter code), scope profile, flags `--steuernummer`/`--bundesland`, env `KIECLI_STEUERNUMMER`/`KIECLI_BUNDESLAND`. Needed only by `kontoabfrage`.
6. A 13-digit ELSTER-format Steuernummer is used as is; a Bescheid-format one is converted with `bundesland` (via `EricMakeElsterStnr` if the spike confirms it, otherwise a table as in haben's `packages/core/src/steuernummer.ts`). Bescheid format without `bundesland` is exit 2 with a `help` naming the flag. The Bescheid format alone is ambiguous (BB, SN and ST share prefix 3 with 11 digits).
7. The Empfänger (Bundesfinanzamtsnummer) is always derived from the Steuernummer, never a setting.
8. Exactly one Steuernummer per call; further Steuernummern of the same Taxpayer go in other profiles or a `--steuernummer` override.

### Output (stdout)

9. Top level: `steuernummer` (ELSTER format), `bundesfinanzamtsnummer`, `abfrageart`, `teile[…]`, plus `test`, `profile`, `warnings` as on every command.
10. Each `teil`: its echoed input (`steuerart`, `ab` + `ein_jahr`, or `jahr`), `bearbeitungshinweis {code, text}`, and `antwort_format: "biene" | "alt"`. Teile are matched to the request by their echoed input, not by position.
11. No merged semantic model; each shape is mapped faithfully, in delivered order:
    - `alt` (`kontoabfrage-output`): `abfrage_tagesdatum`, `steuerarten[{klartext, erlaeuterung, teilbetraege[{zeitraum, faelligkeit, wert, erlaeuterung}], gesamtbetrag}]`, `gesamtsumme`, `erlaeuterung`.
    - `biene` (`kontoabfrage_antwort`): `stpfl {finanzamt, name, steuernummer, abfragedatum, kein_leistungsgebot}`, `zeilen[…]` and `summen[…]` with the fields of `steuerkontoOffeneBetraege` / `steuerkontoTilgungen` / `steuerkontoauszug` (`text`, `faelligkeit_wertstellung`, `betrag`, `zufluss`, `abfluss`, `offen_forderung`, `offen_verbindlichkeit`, `erlaeuterungen`). Text-only rows that act as headings are kept as rows, not turned into groups.
12. Amounts become decimal strings with a dot and no thousands separator (`"3782.00"`), never JSON numbers. Parseable dates become ISO 8601; the BIENE `abfragedatum` has no timezone and stays local time without `Z`. Free-text periods (`4.Vj.2019`) stay text.
13. An empty account (`keine offenen Positionen`, empty `steuerkontoOffeneBetraege`) is success with empty lists.
14. `--mit-xml` adds `xml`, the raw `kontoabfrage` Nutzdaten as one string. Off by default.

### Result codes → exit codes

| ELSTER code (Fehlerliste 2.1.1) | Exit |
|---|---|
| `0` on every teil | 0 |
| `950013001` some teile failed | 7, stdout lists each teil with its code |
| `950013002`, `230077012`, `950013003`, `950017004`, `950017005`–`950017007`, `950017011` | 5 |
| `950017009`, `950017010`, `950019999` | 5, `help`: contact the Finanzamt |
| `950019011`, `950017008` | 6, `retry: true` |

For `230077012`, `notes` explain that the Kontoabfrage presupposes an authorisation (Vollmacht or Freischaltung in Mein ELSTER). `describe` lists the Steuerart values and these codes.

Checks added to [Test-system spike: ERiC end to end](11-test-system-spike.md). Amends [Agent interface](09-agent-interface.md) (the `kontoabfrage` placeholder) and [Configuration and secrets](10-config-and-secrets.md) (the Taxpayer-identifier row).

Amended by [Test-system spike: ERiC end to end](11-test-system-spike.md): the test system never returns a Kontoabfrage result (test Organisationszertifikat: `230077012` for every Steuernummer; personal test certificates: `130025001`), so the `biene`/`alt` mapping is checked only in the live check. `230077012`/`950017011` get a `help` on activating the Steuerkontoabfrage in Mein ELSTER, which varies by Bundesland.
