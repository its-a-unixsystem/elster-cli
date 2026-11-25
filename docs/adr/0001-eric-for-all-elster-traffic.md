# ERiC for all ELSTER traffic, no self-built transport

KIECLI sends every ELSTER request through ERiC (and Postfach attachments through Otto, shipped in the same package), although the Postfach procedures are part of the Offene Schnittstelle and could be implemented directly. The Kontoabfrage and all later sending (UStVA, ZM) are ERiC-only by ELSTER policy, so ERiC is unavoidable; once it is there, a second, self-built transport would duplicate code for no feature. We accept a runtime C library downloaded by the user (`kiecli eric install`), per-version handling of the ERiC API, and no Linux arm64 support, since ELSTER ships no ERiC build for it.

## Considered Options

- **Self-built Offene Schnittstelle for the Postfach, Kontoabfrage moved to a later version**: one static binary with no C library, but ERiC still arrives with sending, leaving two transports to maintain.
- **Self-built transport for everything, including the Kontoabfrage**: technically possible (same `send-Auth` wire format) but not sanctioned by ELSTER, and the penalty is a blocked HerstellerID.
- **Optional ERiC, used only for the Kontoabfrage**: rejected because there is no use case for a Postfach-only installation.

## Consequences

- Rebuilding a pure-Go/Rust transport later is only worth it if ERiC cannot do something v1 needs; none was found.
- The research on CMS, XML-DSig and the `extern-offen.p12` TLS client certificate is background only.
- ERiC cannot be redistributed, so it lives outside the repository and release artifacts; CI downloads it at test time.
