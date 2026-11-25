# ELSTER terms for an open-source client

Type: research
Status: resolved
Blocked by:

## Question

What do ELSTER's terms allow and require for an open-source client used by many people on their own machines?

Cover: whether one HerstellerID may be used by all users of an open-source tool or each user must obtain their own; any obligations attached to a HerstellerID (Testlauf, registration of product versions, reporting); whether the XSD schemata, documentation, example XML files and test certificates may be redistributed in a public repository; whether the product name "elster-cli" or the ELSTER name/logo is restricted; how existing open-source ELSTER clients handle these points.

## Answer

- No accessible source says whether one HerstellerID may serve all users of an open-source tool. Every ELSTER document says only that each participating Hersteller must apply for its own ID (BayLfSt); the ID is checked server-side and tied to blocking. The developer-area terms and the ERiC licence are behind a login and were not read in the original.
- Existing clients split: cotto/csotto take the ID from the user's env var; geierlein hard-codes one project ID. Recommendation: user-supplied ID, validated as five digits, never in code or repo, and the README states the user owns the obligations attached to it.
- Obligation found: Bestätigung within 24 h or risk of blocking the ID holder (SSb Datenabholung 6.4). This conflicts with the map's "Bestätigung only on explicit request".
- Redistribution: no licence text anywhere in the downloads, only "Copyright © ELSTER" footers. The files are publicly downloadable, but that is not a licence. Test certificates must not be passed on (readme). Keep all downloads, test certificates and `extern-offen.p12` out of the repo; fetch at test time or hand-write fixtures.
- Name: ELSTER is a Freistaat Bayern trademark; no rule against "elster-cli" found. Do not use the logo; add a non-affiliation notice.
- Eight questions for ELSTER are listed in the research file.

Details: [research/04-open-source-terms.md](../research/04-open-source-terms.md)

Amended by [Obtain a HerstellerID](06-obtain-herstellerid.md): ELSTER refused the product name "elster-cli" in the HerstellerID application. The project is now KIECLI; the non-affiliation notice and the no-logo rule stand.
