# Ship the Bescheidwertnummer table?

Type: grilling
Status: open
Blocked by: 02, 04

## Question

BDS-2 `W` records carry a Bescheidwertnummer, a format and a value. The meaning of each number (festgesetzte Steuer, Abrechnungsbetrag, Vorauszahlung, …) is defined only in the developer-area `Bescheidnr_*.zip` ([Download developer-area Bescheid material](02-developer-area-material.md)). ERiC does not interpret it ([Bescheiddaten formats per Bereitstellungsdatenart](01-bescheiddaten-formats.md)), and no open-source table exists.

How does elster-cli get the meanings?

- embed the table (or a curated subset for the named key figures) in the open-source binary;
- ship raw `nummer`/`format`/`wert` only and let the user point at their own copy of the zip;
- ask ELSTER first (see [Ask ELSTER the open terms questions](../../elster-cli/issues/12-ask-elster.md)).

The choice hangs on the licence wording ticket 02 records, and only matters if [Redraw the destination after the format finding](04-redraw-destination.md) keeps BDS-2.
