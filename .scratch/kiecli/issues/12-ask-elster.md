# Ask ELSTER the open terms questions

Type: task
Status: resolved
Blocked by:

## Question

Several decisions rest on terms that public sources don't settle. HITL: send these questions to ELSTER as the registered manufacturer (developer area or `id@elster.de`, never a public forum post with personal details) and record the answers here.

The eight questions are drafted under "Open points" in [ELSTER terms for an open-source client](../research/04-open-source-terms.md): redistribution of schemas, examples and docs; the HerstellerID model for an open-source tool; the 24 h Bestätigung rule for a tool that leaves Bestätigung to the caller; Testlauf and product registration; the example ID `74931` and test certificates in CI; the name "elster-cli"; embedding `extern-offen.p12`; ERiC licence terms.

Add one more, from [ERiC and the Kontoabfrage](03-eric-kontoabfrage.md): may a manufacturer send ElsterKontoabfrage through its own implementation of the transport instead of ERiC?

Update after [ERiC for the Kontoabfrage](07-eric-decision.md) (v1 runs entirely on ERiC): the questions on embedding `extern-offen.p12` and on sending ElsterKontoabfrage without ERiC are moot; skip them. The ERiC licence question now matters more: confirm that each user downloading ERiC from `download.elster.de` under their own HerstellerID is within the terms.

Answers feed ERiC for the Kontoabfrage, Configuration and secrets, the testing strategy and distribution. None of those tickets waits on this one; revisit them if an answer contradicts what they decided.

## Answer

**Not sent.** KIECLI is a small open-source tool; v1 proceeds on the readings below without asking ELSTER. If ELSTER ever objects to one of them, reopen the ticket named next to it.

1. Redistribution: nothing from the ELSTER downloads goes into the repository; fixtures are hand-written with invented data (testing strategy).
2. HerstellerID: each user supplies their own; the project holds none in code (Configuration and secrets).
3. 24 h Bestätigung: binds every ID holder collecting real data; the CLI confirms only on explicit request and must make the deadline visible to the caller (Agent interface).
4. Testlauf and product registration: whatever ELSTER requires is part of each user's own HerstellerID application, not the tool's concern.
5. Public CI: test certificates downloaded from `download.elster.de` at test time, never committed; our HerstellerID only as a CI secret (testing strategy).
6. Name: KIECLI, with a non-affiliation notice and no ELSTER logo or look (distribution). ELSTER refused the first name, "elster-cli", in the HerstellerID application ([Obtain a HerstellerID](06-obtain-herstellerid.md)).
7. ERiC licence: the CLI never ships ERiC; each user downloads it through `eric install` after explicit `--accept-eric-terms` (ERiC for the Kontoabfrage).

The drafted questions remain under "Open points" in [ELSTER terms for an open-source client](../research/04-open-source-terms.md) if they ever need asking.
