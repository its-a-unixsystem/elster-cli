# Go vs Rust library support

Type: research
Status: resolved
Blocked by: 01

## Question

For each primitive the transport needs (from ticket 01, plus mTLS for the Objektspeicher), which maintained Go and Rust libraries support it, and where would we have to hand-roll?

Cover at least: PKCS#12 parsing of ELSTER soft-PSE files (including legacy encryption algorithms they may use), CMS/PKCS#7 `EnvelopedData` with RSA-OAEP and AES-256-CBC, `EncryptedData`, signatures as required by `send-Auth`, TLS client certificates, XML building and XSD validation, OS keychain access on Linux and macOS, static builds and cross-compilation for macOS. Verify against the ELSTER test certificates in `docs/API/Test_Zertifikate.zip` where possible.

## Answer

Both languages cover every primitive; Go comes out ahead mainly on build and CMS maturity. Details, versions and evidence in [research/05-go-vs-rust-libraries.md](../research/05-go-vs-rust-libraries.md).

- Verified with the ELSTER test PSEs and `extern-offen.p12`: Go (`go-pkcs12` v0.7.4) and Rust (`p12-keystore` 0.4.1) both read the RC2-40 and PBES2/AES-256 bags, SHA-1 and SHA-256 MACs, both key bags; stdlib or `rsa` 0.10 do OAEP-SHA-256 and PSS-SHA-256; TLS 1.3 with the RSA-4096 client certificate works in both.
- Hand-roll in both: CMS EnvelopedData with OAEP SHA-256/MGF1-SHA-256. `smallstep/pkcs7` (Go) and RustCrypto `cms` both get it wrong or lack it. Go prototype is 89 lines and OpenSSL decrypts it.
- Go decrypts the BER/OAEP/EncryptedData variants and builds the Objektspeicher SignedData with `smallstep/pkcs7` (verified with OpenSSL). Rust's `cms` has no decrypt and only its 0.3 release candidate parses BER.
- XML-DSig (PSS, XPath Filter 2.0): Rust `bergshamra` does it (verified, but a 9-month-old single-maintainer crate); in Go it is about 50 lines on `goxmldsig`'s c14n (verified against libxml2 and OpenSSL).
- Pure-Go and pure-Rust XSD validators both validate the ELSTER schema set.
- Builds: Go `CGO_ENABLED=0` cross-builds to darwin (verified). Rust `rustls` (aws-lc-rs/ring) fails to cross-build on a plain Linux host; it needs zig, osxcross or native macOS CI.
- macOS runtime was not tested; Caveat: in `go-pkcs12` the multi-key PSEs need the deprecated `ToPEM`.
