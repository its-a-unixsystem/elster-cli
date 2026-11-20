# Open-interface transport, end to end

Type: research
Status: resolved
Blocked by:

## Question

What exactly must a client do, byte for byte, to send a `send-Auth` request over the Offene Schnittstelle and read the response, without ERiC?

Cover: building the ElsterXML (TransferHeader, NutzdatenHeader, Nutzdaten), GZIP, CMS `EnvelopedData` (RSA-OAEP + AES-256-CBC) against the Clearingstelle key, `CMSEncryptedData` vs `CMSEnvelopedData` and the `Transportschluessel`, how authentication/signing works for `send-Auth` (see the Authentifizierung and ELSTER-Token PDFs), Base64, the HTTPS POST (endpoints, TLS pinning of `datenannahme.elster.de`), decrypting and validating the response, error reporting (`Fehlerliste.xml`). Also: test vs live system (Testmerker, endpoints), and whether the example HerstellerID 74931 may be used for tests before our own HerstellerID arrives.

Output: an ordered list of steps with the cryptographic primitives each needs, so ticket 05 can check library support.

## Answer

A `send-Auth` request is an ElsterXML document in which `SigUser`, `DatenLieferant`, `TransportSchluessel` and `DatenTeil` hold `gzip → CMS → Base64` payloads. CMS is `EnvelopedData` for the Clearingstelle key (RSA-3072, RSAES-OAEP with SHA-256 and MGF1-SHA-256, content AES-256-CBC). `SigUser` carries an XML-DSig over the already encrypted `DatenTeil` (RSASSA-PSS, SHA-256, c14n 1.0, XPath Filter 2.0), made with the Certificate's signature key.
- The reply comes back as `CMSEncryptedData` (AES-256-CBC with the request's session key, no client certificate needed) or as `CMSEnvelopedData` to the certificate sent in `TransportSchluessel` (Base64 text of the DER certificate). Replies are BER with indefinite lengths.
- Transport is an HTTPS POST to `datenannahme1..4.elster.de/ERiC_IAS/ERiClet`. A live probe showed that the server requires a TLS client certificate; the public `extern-offen.p12` (password `change_me`) from the download package works. The shipped server certificate has expired, so pin nothing from the zip.
- Test and live share the endpoints; a `Testmerker` marks test cases. With `extern-offen`, only Testmerker `700000001` was accepted for `PostfachStatus`.
- The example HerstellerID 74931 is rejected exactly like random IDs (`HerstellerID ist nicht zugelassen`), so a real HerstellerID is needed even for tests. The PSS salt length and the exact bytes of the compressed `DatenTeil` are unverified because that check fails first.

Full steps, primitives list and open points: [research/01-open-interface-transport.md](../research/01-open-interface-transport.md)
