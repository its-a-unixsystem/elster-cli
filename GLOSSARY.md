# elster-cli

A command-line client that retrieves a single taxpayer's data from ELSTER, the German tax administration's electronic interface, for use by AI agents.

## Language

### Parties and credentials

**Taxpayer**:
The organisation or person whose ELSTER data a call retrieves, identified by the Certificate the user supplies.
_Avoid_: Mandant, client, account, user

**Certificate**:
The taxpayer's ELSTER soft-PSE file (`.pfx`) plus its PIN, issued through Mein ELSTER, that authenticates and signs every request.
_Avoid_: Zertifikat, key, login

**Konto-ID**:
The identifier of the Mein ELSTER account a Certificate was issued for; a renewed Certificate keeps it.
_Avoid_: account ID, user ID, ELSTER ID

**Organisationszertifikat**:
A certificate issued to an organisation rather than a natural person.
_Avoid_: company cert, org cert

**HerstellerID**:
The manufacturer identifier the Bayerisches Landesamt für Steuern assigns to a registered software maker; every request carries one.
_Avoid_: vendor ID, producer ID

### Retrieval

**Postfach**:
The taxpayer's Mein ELSTER mailbox, in which the tax administration provides assessments and messages for collection.
_Avoid_: inbox, mailbox

**Postfach-Nachricht**:
One item provided in the Postfach, of a given Bereitstellungsdatenart.
_Avoid_: message, document, letter

**Bereitstellungsdatenart**:
The kind of a Postfach-Nachricht, such as `EPBescheid`, `EPMitteilung` or a `DIVA*` type.
_Avoid_: message type, category

**Datenpaket**:
The main content of a Postfach-Nachricht, a text or XML document.
_Avoid_: payload, body

**Anhang**:
A file attached to a Postfach-Nachricht, such as the PDF of a Bescheid.
_Avoid_: attachment, document

**Verwaltungsakt**:
Any decision a Finanzamt or Kommune issues to the taxpayer through the Postfach, such as a Bescheid, a Stundung, an Erlass or an Aussetzung der Vollziehung.
_Avoid_: administrative act, decision, notice

**Bescheid**:
The Verwaltungsakt by which a Finanzamt or Kommune assesses a tax for a Steuerart and period, provided to the taxpayer as a Postfach-Nachricht.
_Avoid_: assessment, tax notice, decision

**Bescheiddaten**:
The machine-readable content of a Bescheid, as opposed to the printed Bescheid in its PDF Anhang; may arrive as the Datenpaket, as an XML Anhang, or as a file embedded in the PDF.
_Avoid_: structured data, assessment data

**Bescheidkennzeichnung**:
The marking of a Verwaltungsakt as erstmalig endgültig, unter Vorbehalt der Nachprüfung, vorläufig or Berichtigung, or a combination of these; the same concept whichever source carries it.
_Avoid_: Vorläufigkeitsmerker, status, flag

**Anfrage**:
Asking the Postfach for the metadata of its Postfach-Nachrichten, without collecting their content.
_Avoid_: list, query, poll

**Abholung**:
Collecting the Datenpaket and Anhänge of a Postfach-Nachricht from the Objektspeicher; obliges the taxpayer to a Bestätigung within 24 hours.
_Avoid_: fetch, download

**Bestätigung**:
The taxpayer's notice to ELSTER that a Postfach-Nachricht has been collected.
_Avoid_: ack, confirm, mark as read

**Kontoabfrage**:
A query of the taxpayer's tax account at a Finanzamt for open amounts, payments made, or amounts assessed.
_Avoid_: balance check, account statement

**Abfrageart**:
The kind of a Kontoabfrage: offene Beträge (`O`), geleistete Zahlungen (`I`) or zum Soll gestellte Beträge (`ZS`).
_Avoid_: query type, request kind

**Sollstellung**:
An amount the Finanzamt has assessed as owed for a Steuerart and year, the subject of a `ZS` Kontoabfrage.
_Avoid_: assessment, charge

**Steuerart**:
The tax a Kontoabfrage is limited to, such as ESt, USt or LSt, in ELSTER's spelling.
_Avoid_: tax type, tax kind

**BIENE Steuerkontoauszug**:
The tax-account system replacing the old Steuerkonto one Finanzamt at a time; a migrated Finanzamt answers a Kontoabfrage in its new response format.
_Avoid_: BIENE, new format

### Identifiers

**Steuernummer**:
The tax number a Finanzamt assigns to a taxpayer, either in the Bescheid format the taxpayer sees (varies by Bundesland) or in the uniform 13-digit ELSTER format.
_Avoid_: tax ID, StNr

**Bundesfinanzamtsnummer**:
The four-digit nationwide number of a Finanzamt; the first four digits of an ELSTER-format Steuernummer.
_Avoid_: Finanzamtsnummer, BUFA, tax office ID

### Transport

**Offene Schnittstelle**:
The subset of ELSTER procedures a client may implement itself, without ERiC.
_Avoid_: open API, open interface

**ERiC**:
The closed-source ELSTER Rich Client library, which ELSTER distributes to registered manufacturers, for building, signing, encrypting and sending ELSTER requests.
_Avoid_: Elster client, Elster SDK

**Testmerker**:
The marker in a request that tells ELSTER to treat it as a test case: processed, but not acted on by the tax administration. Requests made with test certificates must carry it.
_Avoid_: test mode, sandbox, test environment

**Objektspeicher**:
ELSTER's object store (OTTER), from which large attachments of a Postfach-Nachricht are fetched.
_Avoid_: OTTER, blob store, attachment server

**Otto**:
The closed-source ELSTER library, shipped in the ERiC package, for fetching objects from the Objektspeicher.
_Avoid_: OTTER client, download library
