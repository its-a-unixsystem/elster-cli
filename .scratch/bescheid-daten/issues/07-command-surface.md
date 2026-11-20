# Command surface for Bescheiddaten

Type: grilling
Status: open
Blocked by:

## Question

Where does the Bescheiddaten JSON appear in elster-cli? The three sources are fixed by [Redraw the destination after the format finding](04-redraw-destination.md): the BDS-2 Datenpaket of `ESB`/`USB`/`GSB`, the XUnternehmen.Gewerbesteuer XML embedded in a `Gewerbesteuerbescheid`, and the decoded DIVA2 metadata.

- a field per Nachricht in the `postfach abholung` output (v1 shape in [Agent interface](../../elster-cli/issues/09-agent-interface.md));
- a separate offline command over a Datenpaket or Anhang already written to disk (e.g. `bescheid lesen <file>`), needing no Certificate or ERiC;
- both.

Settle with it: whether the decoded DIVA2 metadata belongs in `postfach anfrage` (where the raw `meta` already lives), in `abholung --mit-meta`, or in both; how unknown XML Anhänge are reported; whether a parse failure of one Nachricht's Bescheiddaten changes `status` or exit code, given the raw files were written fine; and what `describe` documents.
