# Go vs Rust: library support for the transport primitives

Research for ticket `issues/05-go-vs-rust-libraries.md`. Input: `research/01-open-interface-transport.md` ("Primitives needed", "Open points") and `research/02-postfach-lifecycle.md` (Objektspeicher signature). Date of research: 2026-11-19.

## How to read this

Evidence tags:

- **[ran]** I wrote a throwaway program and ran it on this machine (Linux x86_64, Go 1.27.4, cargo/rustc 1.89.0, OpenSSL 4.1.0 as the independent oracle). The programs live outside the repo in the scratchpad `r05/` directory (`go/t2..t11`, `rs/src/bin`, `rs2`, `rs5`). Inputs were only the public test material: the three test PSEs from `docs/API/Test_Zertifikate.zip` (PIN `123456`), `extern-offen.p12` (password `change_me`), the Clearingstelle certificate, and the XSDs and sample XML from the ELSTER downloads. Nothing was sent to any ELSTER server. No key or certificate was copied into the repo.
- **[src]** Read in the library's source, docs or changelog, not executed.
- **[api]** Version and date from the Go module proxy (`proxy.golang.org`), the crates.io API, or the GitHub API (queried 2026-11-19).

"Support" values: **supported** (library does it, verified or documented), **partial** (works with caveats or a deprecated API), **hand-roll** (we write it on top of lower-level primitives).

Not verified anywhere: macOS runtime behaviour (no Mac available), any request against ELSTER, live Keychain writes.

## Verdict in one paragraph

Both languages can do everything, and both need the same two hand-rolled pieces: CMS `EnvelopedData` with RSAES-OAEP(SHA-256/MGF1-SHA-256) and XML-DSig with RSASSA-PSS plus the XPath Filter 2.0 template. Go's gaps are smaller in effort because the stdlib covers RSA, X.509 (including PSS-signed certificates), TLS 1.3 with the RSA-4096 client certificate and gzip, and `smallstep/pkcs7` already decrypts the BER/OAEP/EncryptedData variants and produces the SignedData. Rust has one library, `bergshamra`, that does the XML-DSig profile in full (verified), which Go lacks, but its CMS crate (`cms`) has no OAEP at all, cannot decrypt, and its only BER-capable version is a release candidate. The decisive difference is the build: Go cross-builds to macOS with `CGO_ENABLED=0` out of the box (verified), while the default Rust TLS stack (`rustls` with `aws-lc-rs`, or `ring`) needs a C compiler for the target and fails on a plain Linux host (verified). ERiC FFI is mentioned only (see end).

## Table: primitive x {Go, Rust}

Primitive numbers follow "Primitives needed" in `01-open-interface-transport.md`.

| # | Primitive | Go: library, version | Go status and evidence | Rust: library, version | Rust status and evidence |
| --- | --- | --- | --- | --- | --- |
| 1 | PKCS#12 read: PBES2/AES-256 key bags, `pbeWithSHA1And40BitRC2-CBC` cert bags, MAC SHA-1 (100000 it.) and SHA-256, bag attributes, two key bags (`signaturekey`, `encryptionkey`) | `software.sslmate.com/src/go-pkcs12` v0.7.4 (2026-10-10) [api] | **partial.** [ran] `ToPEM` decodes all three test PSEs (8 blocks each: 2 keys, 6 certs, `friendlyName` and `localKeyId` preserved) and `DecodeChain` decodes `extern-offen.p12` (RSA-4096, SHA-256 MAC, PBES2). `DecodeChain` refuses the test PSEs ("expected exactly one key bag"). `ToPEM` is marked `Deprecated` (writes raw PKCS#1 under a "PRIVATE KEY" label; I had to use `x509.ParsePKCS1PrivateKey`) [src: `pkcs12.go` L311-317]. DER only, no BER [src: README]. | `p12-keystore` 0.4.1 (2026-10-13, default feature `pbes1` pulls `rc2`, `des`, `cbc`) [api][src: `Cargo.toml`] | **supported.** [ran] `Pkcs12Archive::from_pkcs12` returns every key and cert bag with `friendly_name` and `local_key_id`; all three test PSEs (2 keys of 3072 bit, 6 certs) and `extern-offen.p12` (4096 bit) parse. The raw-archive API exists exactly for multi-key files [src: `archive.rs`]. RustCrypto `pkcs12` is at 0.1.0 and its 0.2.0 has blockers open (issue #2387) [api]. |
| 2 | X.509 parse, RSASSA-PSS-signed certs, RSA-3072/4096, issuer DN, serial | stdlib `crypto/x509` | **supported.** [ran] All certs of the test PSEs parse (`SHA256-RSAPSS`), `extern-offen` (SHA384-RSA) and the Clearingstelle `FrontendKey` (PSS, KU `digitalSignature` only) parse; `cert.RawIssuer` and `SerialNumber` give the `IssuerAndSerialNumber` parts. | `x509-cert` 0.3.0 (2026-07-09) [api], `rsa` 0.10.0 | **supported.** [ran] All certs of the test PSEs parse; signature algorithm OID `1.2.840.113549.1.1.10` (PSS) is exposed; `extern-offen` parses. The `rsa` crate does not enforce Key Usage (OAEP with a key from a `digitalSignature`-only cert is just an RSA op) [src]. |
| 3 | RSA-OAEP SHA-256 / MGF1-SHA-256, empty label, encrypt and decrypt | stdlib `crypto/rsa` | **supported.** [ran] `EncryptOAEP`/`DecryptOAEP` with `sha256.New()` round-trip on the 3072-bit key; encrypting to the `FrontendKey` cert (no keyEncipherment) works. | `rsa` 0.10.0 (2026-11-08) [api] | **supported.** [ran] `oaep_decrypt::<Sha256>` decrypts a blob made by `openssl pkeyutl -pkeyopt rsa_oaep_md:sha256 -pkeyopt rsa_mgf1_md:sha256`. 0.10.0 is the release that replaces `num-bigint-dig` with constant-time `crypto-bigint` and addresses the Marvin side channel (RUSTSEC-2023-0071 history) [src: `CHANGELOG.md`, README]; it is 11 days old. |
| 4 | AES-256-CBC + PKCS#7 padding, secure random | stdlib `crypto/aes`, `crypto/cipher`, `crypto/rand` | **supported.** [ran] used in the hand-rolled EnvelopedData (item 5) and decrypted by `openssl cms`. | `aes` 0.9.4, `cbc` 0.2.1 (RustCrypto) [api] | **supported** [src]. Not exercised end to end because no stable CMS builder uses it with OAEP (item 5). |
| 5 | CMS `EnvelopedData` **generation**: KeyTransRecipientInfo, RSAES-OAEP-params (SHA-256, MGF1-SHA-256, pSpecified empty), AES-256-CBC | `github.com/smallstep/pkcs7` v0.2.3 (2026-07-29) [api] | **hand-roll.** [ran] `smallstep/pkcs7.Encrypt` with `KeyEncryptionAlgorithm = OAEP`, `KeyEncryptionHash = SHA256` emits OAEP parameters with only the hash (MGF1 defaults to SHA-1); `openssl cms -decrypt` fails with "bad decrypt". Cause [src: `encrypt.go` L367-396]: `getParametersForKeyEncryptionAlgorithm` sets `HashFunc` and never `MaskGenFunc`. A 89-line hand-rolled encoder with `encoding/asn1` + `crypto/rsa` [ran: `go/t8`] is decrypted by `openssl cms -decrypt` using the test PSE's encryption key. | `cms` 0.2.3 (2026-11-09) and 0.3.0-rc.2 [api] | **hand-roll.** [src] The `KeyTransRecipientInfoBuilder` hard-codes `Pkcs1v15Encrypt` (`src/builder.rs` L796 in 0.3.0-rc.2); `grep -i oaep` over `src` and `tests` of both versions finds nothing. AES-256-CBC content encryption exists in the builder. The OAEP parameter types exist in the `pkcs1` crate [src, not run]. |
| 6 | CMS parse/decrypt of `EnvelopedData` and `EncryptedData`, **BER indefinite length**, external key for `EncryptedData` | `smallstep/pkcs7` v0.2.3 | **supported.** [ran] `Parse` + `Decrypt` (OAEP SHA-256, from `openssl cms -encrypt ... -stream`, i.e. indefinite-length BER) and `DecryptUsingPSK` (`openssl cms -EncryptedData_encrypt -secretkey ... -aes-256-cbc`, DER and BER) all return the plaintext. Has `ber2der` [src: `ber.go`]. `digitorus/pkcs7` (latest 2026-11-04) has the same shape (`Decrypt`, `DecryptUsingPSK`, `ber2der`) [src, not run]. | `cms` 0.2.3 (stable) / 0.3.0-rc.2 on `der` 0.8 | **hand-roll** (no decrypt API in either, [src] `grep decrypt`). Parsing: [ran] `cms` 0.2.3 `ContentInfo::from_der` fails on the OpenSSL streaming output with `indefinite length disallowed`. With `cms` 0.3.0-rc.2 and `der` 0.8, `ContentInfo::from_ber` accepts it, but `decode_as::<EnvelopedData>` still fails; re-encoding the content `Any` and calling `EnvelopedData::from_ber` / `EncryptedData::from_ber` works, one-chunk case only. Then decrypt with `rsa` OAEP + `cbc`. |
| 7 | RSASSA-PSS SHA-256, MGF1-SHA-256, salt 32, key from PKCS#12 | stdlib `rsa.SignPSS` | **supported.** [ran] signature verified by `openssl dgst -sigopt rsa_pss_saltlen:32 -sigopt rsa_mgf1_md:sha256`. | `rsa` 0.10.0 `pss::SigningKey::<Sha256>` (default salt = digest size) | **supported.** [ran] signature verified by the same `openssl` command. |
| 8 | XML-DSig: Canonical XML 1.0 inclusive, SHA-256, PSS (`xmldsig-more#sha256-rsa-MGF1`), XPath Filter 2.0 template | `github.com/russellhaering/goxmldsig` v1.6.1 (2026-08-04), `github.com/beevik/etree` v1.8.1 [api] | **hand-roll on top of the library's c14n.** [src] goxmldsig signs only `rsa-sha1/256/384/512` (`xml_constants.go`), no PSS, no filter2. [ran] Its `MakeC14N10RecCanonicalizer` gives byte-identical output to libxml2 (`lxml` c14n) for the `DatenTeil` subtree including the inherited `xmlns` declarations and `&#xD;` escaping, and for `SignedInfo`. With `crypto/rsa` `SignPSS` the whole signature (digest, c14n, sign) is about 50 lines; digest equals libxml2's and `openssl dgst` verifies the PSS signature [ran: `go/t7`]. | `bergshamra` 0.12.0 (2026-11-09, repo created 2026-02-22, 12 stars) [api] | **supported (young).** [ran] A template with `SignatureMethod …#sha256-rsa-MGF1`, Reference `URI=""`, filter2 `intersect` `//elsterDefaultNs:DatenTeil` and c14n 1.0 is signed by `bergshamra_dsig::sign::sign` with the test key. The `DigestValue` equals libxml2's c14n+SHA-256 of `DatenTeil`; the `SignatureValue` verifies with `openssl` PSS (salt 32 and auto) over libxml2's c14n of `SignedInfo`. README: c14n 1.0/1.1/exc, XPath Filter 2.0, RSA-PSS, 1064/0 xmlsec interop tests pass, no C code [src]. `xml_c14n` (0.3.0) was last updated 2023-11-29 [api]. |
| 9 | CMS `SignedData` for the Objektspeicher `objektid` (SHA-256, `signingTime` + `messageDigest`, cert embedded) | `smallstep/pkcs7` v0.2.3 | **supported.** [ran] `NewSignedData` + `AddSigner(sigCert, sigKey)` with SHA-256 produces a structure that `openssl cms -verify -noverify` accepts and that contains `contentType`, `signingTime`, `messageDigest` attributes and the certificate. `signatureAlgorithm` is `sha256WithRSAEncryption`, whereas the OTTER sample shows `rsaEncryption` (02-postfach-lifecycle, SSb-OTTER 5.5); whether the server cares is untested. Attribute order and the `contentType` attribute need a check against the sample. `digitorus/pkcs7` also signs with RSA-PSS [src, not run]. | `cms` 0.3.0-rc.2 `SignedDataBuilder` + `SignerInfoBuilder` [src: `tests/builder.rs` L81-160 build RSA PKCS#1 v1.5, ECDSA and RSA-PSS signers, `create_signing_time_attribute`, `create_message_digest_attribute` exist in `src/builder.rs`] | **supported on the release candidate, not run.** 0.2.3 has the same builder on `rsa` 0.9 / `der` 0.7. Mixed-version friction below. |
| 10a | XML build/parse, namespaces, ISO-8859-15 input | stdlib `encoding/xml`, `golang.org/x/text/encoding/charmap` (`ISO8859_15`) [src: `tables.go`], `etree` | **supported.** [ran] `etree` parses the sample and c14n works; the charset reader is a one-liner [src]. | `quick-xml` 0.42.0, `roxmltree` 0.21.2 [api]; `encoding_rs` for ISO-8859-15 | **supported** [src, charset path not run]. |
| 10b | XSD validation of the headers (optional) | `github.com/jacoelho/xsd` v0.6.14 (2026-11-15, pure Go XSD 1.0) [api] | **supported (young).** [ran] Compiles `elster11_datenabholung_32.xsd` (with the `Schemata/header/*` includes) and validates a corrected SSb-DA sample request; the unmodified sample is rejected for `HerstellerID` with the same facet violation as `xmllint` (`length 5`, `\d{5}`). Alternatives need libxml2 via cgo (`krolaw/xsd`, `terminalstatic/go-xsd-validate`, `lestrrat-go/libxml2`) [src: package docs]. | `uppsala` 0.12.1 (2026-11-09; also the XML layer of `bergshamra`) [api] | **supported (young).** [ran] `XsdValidator::from_schema_with_base_path` compiles the same schema set; same verdicts as `xmllint` on both files. `libxml` (0.3.24) wraps C libxml2 [api]. Note: `xmllint` itself fails to compile `th000011_extern.xsd` from the XML-Schnittstelle zip alone (duplicate type definitions); the Datenabholung zip's `elster11_datenabholung_32.xsd` wrapper compiles. |
| 11 | HTTPS client, TLS 1.2/1.3, SNI, client cert from PKCS#12 (RSA-4096), system trust store or pins | stdlib `crypto/tls`, `net/http` | **supported.** [ran] In-process TLS 1.3-only server (`httptest`, `RequireAnyClientCert`) accepts the `extern-offen` RSA-4096 client certificate loaded from the p12 (negotiated `0x0304`). System roots on macOS without cgo use the in-tree `crypto/x509/internal/macos` binding [src: `root_darwin.go`]. | `rustls` 0.23.46 (2026-11-12) with `aws-lc-rs` 1.19.0 (default) or `ring` 0.17.14 [api] | **supported.** [ran] Loopback test, server and client both rustls: client auth with the PKCS#8 key and cert from `p12-keystore`, TLS 1.3, `peer_certificates` = 1. Platform trust store needs `rustls-platform-verifier` or bundled roots (not tested). The server list in 02 includes `TLS_AES_128_CCM_SHA256`; neither Go nor rustls offers CCM, GCM suites suffice (02 lists them first). |
| 12 | SHA-256, SHA-1, MD5 (send-NoSig only) | stdlib | **supported.** | `sha2`, `sha1`, `md-5` (RustCrypto) | **supported** [src]. |
| 13 | gzip, Base64 | stdlib `compress/gzip`, `encoding/base64` | **supported.** | `flate2` 1.1.11, `base64` | **supported** [api]. |
| 14 | OS keychain: Linux Secret Service, macOS Keychain | `github.com/zalando/go-keyring` v0.2.8 (2026-03-23; last push 2026-11-17) [api] | **supported.** [ran] `keyring.Get` on this Linux desktop reaches the Secret Service over D-Bus (`secret not found in keyring`), pure Go via `godbus`. [ran] `CGO_ENABLED=0` builds for `darwin/arm64`, `darwin/amd64`, `linux/arm64` succeed. macOS backend shells out to `/usr/bin/security` [src: `keyring_darwin.go` L29]. `99designs/keyring` v1.2.2 last released 2022-12 [api]. | `keyring` 4.2.0 (2026-08-29; default feature `v1` = `apple-native-keyring-store/keychain` + `zbus-secret-service-keyring-store` + Windows) [api][src: `Cargo.toml`] | **supported.** [ran] `Entry::get_password` on this Linux desktop returns `NoEntry` from the Secret Service (pure-Rust `zbus` path). macOS store uses Security.framework bindings (links Apple frameworks, so cross-building needs an SDK, see below). Not exercised on macOS. |

## Cross-compilation and static builds

Test host: Linux x86_64 with gcc and clang, no macOS SDK, no `zig`, no `osxcross`.

**Go**

- [ran] `CGO_ENABLED=0 GOOS=darwin GOARCH=arm64 go build` of the program that uses `go-pkcs12`, `crypto/x509`, `crypto/rsa` succeeds, and `go-keyring` builds for `darwin/arm64`, `darwin/amd64` and `linux/arm64` with `CGO_ENABLED=0`. Every library in the table above except the libxml2-based XSD validators is pure Go. Result: one `go build` per target, static binary on Linux, no Apple SDK needed on the build host.
- [src] Go's macOS system-root loader and Keychain-less TLS do not need cgo (`crypto/x509/internal/macos`).
- Runtime on macOS is untested.

**Rust**

- [ran] `rustup target add aarch64-apple-darwin` (user-level, no sudo) followed by `cargo check --target aarch64-apple-darwin`:
  - Pure-RustCrypto set (`p12-keystore`, `rsa`, `x509-cert`, `sha2`, `cbc`, `aes`) passes (`rs3`); `cms` 0.2.3 + `bergshamra` + `uppsala` passes (`rs2`).
  - `rustls` with default features fails: `aws-lc-sys` and `ring` run the host `cc` with `-arch arm64 -mmacosx-version-min=11.0`, which GCC rejects ("unrecognized command-line option '-arch'").
  - So Rust cross builds need an Apple-aware C toolchain (`cargo-zigbuild`/`zig cc`, `osxcross`, or building on macOS CI runners) and also a macOS SDK to link Security.framework (`keyring`). I did not try these.
- [src] `rustls-rustcrypto` (pure Rust provider) is at `0.1.0-pre.0` (2026-11-09) [api], so it is not a production alternative today.
- Static Linux builds: Rust `x86_64-unknown-linux-musl` is possible with `rustls`; I did not build it. Go static is the default with `CGO_ENABLED=0`.
- CI shortcut that avoids all of this for either language: build each target on its native GitHub runner (`ubuntu`, `macos-14`). That costs nothing in code but loses "cross-build from one machine".

**C dependency summary**

| Option | C dependency | Effect |
| --- | --- | --- |
| Go stdlib + `go-pkcs12` + `smallstep/pkcs7` + `goxmldsig` + `jacoelho/xsd` + `go-keyring` | none | `CGO_ENABLED=0` everywhere [ran] |
| Go + libxml2 wrappers for XSD | libxml2 via cgo | breaks the easy macOS cross build [src] |
| Rust `rustls` default | `aws-lc-sys` (C, cmake/cc) | fails to cross [ran] |
| Rust `rustls` + `ring` | `ring` C/asm via `cc` | fails to cross the same way [ran] |
| Rust `openssl` crate | OpenSSL (system or vendored) | not tested; also OpenSSL 3+/4 refuses RC2-40 in the default provider [ran: `openssl pkcs12 -info` on the test PSE prints `unsupported ... RC2-40-CBC` until `-legacy`], so a PKCS#12 route via OpenSSL needs the legacy provider |
| Rust `libxml` | libxml2 | avoid |

## Hand-roll effort per gap

Estimates are for one developer who already has the test vectors, including tests against `openssl cms`/`dgst` as oracle, not including review.

| Gap | Go | Rust |
| --- | --- | --- |
| Read both key bags of a PSE | Wrap deprecated `ToPEM` (about 40 lines, then `ParsePKCS1PrivateKey`), or vendor the `go-pkcs12` decoder (1-2 days). Risk: `ToPEM` may be removed in a future release; pin the version. | none (`Pkcs12Archive`) |
| CMS EnvelopedData with OAEP-SHA-256 | 0.5 day. Prototype is 89 lines and passes `openssl cms -decrypt` [ran]. Alternative: patch `smallstep/pkcs7` to set `MaskGenFunc` (small, upstream has no open issue about it; 14 open issues in total [api]). | 1-1.5 days: define `KeyTransRecipientInfo` with `RsaOaepParams` from `pkcs1` on the `cms`/`der` structs, AES-256-CBC with `cbc`, assemble `ContentInfo`. Not prototyped. |
| CMS Enveloped/Encrypted decrypt, BER | 0 days (library), 1-2 days if we drop the dependency and write BER to DER (about 150 lines, reference: `ber.go` of `smallstep/pkcs7`). | 1-1.5 days: BER normalisation via `der` 0.8 `from_ber` on a release candidate (or own converter), then OAEP + CBC. Multi-chunk constructed OCTET STRING not tested. |
| CMS SignedData | 0 days (verified), or 0.5 day by hand. | 0-0.5 day if `cms` 0.3 final lands; otherwise pin the release candidate or hand-roll (1 day). |
| XML-DSig PSS + filter2 | 0.5 day. Fixed template, c14n from `goxmldsig`, digest and `SignPSS` [ran]. | 0 days with `bergshamra` (verified), or 1 day hand-rolled (c14n 1.0 is the hard part; crate `xml_c14n` is unmaintained since 2023). |
| XSD | 0 (pure Go validator exists) | 0 (`uppsala`) |

The two shared unknowns that affect either language are in `01-open-interface-transport.md` Open points 2 and 3 (PSS salt length, plain content of `DatenTeil`/`SigUser`); no library choice resolves them.

## Maintenance snapshot

All dates from the GitHub/crates.io/proxy APIs, 2026-11-19.

| Library | Latest release | Notes |
| --- | --- | --- |
| `go-pkcs12` | v0.7.4, 2026-10-10 | 17 open issues, not archived; DER only. |
| `smallstep/pkcs7` | v0.2.3, 2026-07-29 | 14 open issues, e.g. #65 "Missing decryption logic path for AES128CBC" (2026-11-10), #54 RSAPSS OID. Fork of `mozilla-services/pkcs7`; the original `go.mozilla.org/pkcs7` v0.10.0 (2026-07-21) has no OAEP [src: `grep` found none]. |
| `digitorus/pkcs7` | pseudo-version 2026-11-04 | 3 open issues; README: Go 1.27+, also ML-KEM/ML-DSA recipients. 2 stars. Not run. |
| `goxmldsig` | v1.6.1, 2026-08-04 | 51 open issues; used mostly for SAML verification. |
| `jacoelho/xsd` | v0.6.14, 2026-11-15 | Requires Go 1.27+, pre-1.0, very active. |
| `go-keyring` | v0.2.8, 2026-03-23 | 1368 stars, pushed 2026-11-17. |
| `p12-keystore` | 0.4.1, 2026-10-13 | 0 open issues, 10.5M downloads [api]. |
| `rsa` | 0.10.0, 2026-11-08 | Fresh major release; `cms` 0.2.3 (stable) still builds on older `der`/`rsa`/`x509-cert` (observed: the cargo registry fetched `x509-cert` 0.2.5 and 0.3.0 side by side), so the stable CMS crate cannot share types with `rsa` 0.10 / `x509-cert` 0.3. |
| `cms` | 0.2.3 (2026-11-09) / 0.3.0-rc.2 | Maintainers said stable releases start with the "actual cryptography crates first" [src: RustCrypto/formats #779 thread]. |
| `bergshamra`, `uppsala` | 0.12.0 / 0.12.1, 2026-11-09 | One maintainer, repo from 2026-02, 4 open issues. Fast moving; pin exact versions. |
| `rustls` | 0.23.46, 2026-11-12 | Mainstream. |
| `keyring` | 4.2.0, 2026-08-29 | Moved to `open-source-cooperative/keyring-rs`; split into `keyring-core` plus per-OS store crates. |

## ERiC (mention only)

If ERiC stays in scope later: Rust has `eric-bindings` and `eric-sdk` (crates.io 0.7.1 and 0.7.2, 2026-08-31 [api]); Go has no ERiC binding (see `03-eric-kontoabfrage.md` lines 128-129 and 171). Without ERiC in v1 this does not influence the choice.

## Open points

1. **Go PKCS#12 on a deprecated API.** The only way to read a PSE with two key bags in `go-pkcs12` is `ToPEM`, which is deprecated. Decide between pinning, vendoring the decoder, or asking upstream for a non-deprecated multi-key API. No open upstream issue was checked.
2. **`smallstep/pkcs7` health.** One maintainer-organisation fork, v0.2.3, 14 open issues; its OAEP encrypt path is wrong for MGF1-SHA-256 (verified), so we would use it for decrypt and SignedData only, and own an encoder. A pure hand-rolled alternative (BER to DER plus asn1 structs) is about 2 days.
3. **OTTER signature details unverified.** `smallstep/pkcs7` emits `sha256WithRSAEncryption` and a `contentType` attribute; the OTTER sample shows `rsaEncryption` and an attribute order the spec calls significant (02-postfach-lifecycle). Whether the Objektspeicher accepts either form is only testable with a real HerstellerID and Certificate.
4. **Rust `cms` crate transition.** The version that can do BER (`der` 0.8 `from_ber`) and the new `rsa` 0.10 is a release candidate. A 2026-Q4 choice of Rust would pin pre-releases or hand-roll the CMS layer.
5. **Young XML libraries on both sides.** `bergshamra`, `uppsala` (Rust) and `jacoelho/xsd` (Go) are less than a year old with single maintainers. The XSD step is optional; the DSig step is small enough to own in Go (50 lines) but not in Rust unless `bergshamra` is used.
6. **Multi-chunk BER.** My BER test inputs have a single OCTET STRING chunk (OpenSSL `-stream` on 21 bytes). ELSTER's real response, with chunked constructed OCTET STRINGs, should be tested against the chosen decoder with a large sample; a payload of several MB could be built with `openssl cms -stream` to approximate it.
7. **macOS is unverified.** Everything about macOS (Keychain backend behaviour, trust store, notarisation, Gatekeeper for an unsigned CLI) is read in source or inferred from successful cross builds. A Mac or CI runner is needed before the language decision is final.
8. **Secret Service availability.** Both Linux backends need a running D-Bus session with a keyring daemon; headless servers and containers usually have none. The design needs a fallback (env var, file with permissions, or `pass`-style); not a library question, but both libraries fail the same way there (the Go one returned a not-found error only because a daemon was present).
9. **Server trust anchors.** Go uses the OS store on macOS without cgo; Rust needs a choice between `rustls-platform-verifier` and bundled Mozilla roots. Whether D-TRUST roots used by `datenannahme.elster.de` are in the Mozilla bundle was not checked (relates to 01 Open point 8).
10. **Test-PSE key size.** The test PSEs hold RSA-3072 keys, not 4096; only `extern-offen.p12` (TLS) is RSA-4096. 01 mentions 3072/4096 in different places; both sizes were exercised.
