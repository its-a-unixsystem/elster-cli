# Go or Rust

Type: grilling
Status: resolved
Blocked by: 05, 07

## Question

Which language does v1 use, now that [ERiC for the Kontoabfrage](07-eric-decision.md) put all ELSTER traffic through ERiC and Otto?

The transport-primitive comparison in [Go vs Rust library support](05-go-vs-rust-libraries.md) no longer decides it (no CMS, XML-DSig or TLS client certificate code of our own). What matters now: loading `libericapi` and `libotto` at runtime with hand-written declarations and no build-time ERiC dependency (Go `purego` without cgo vs Rust `libloading`), passing ERiC structs and return buffers safely, switching function signatures per ERiC major, building release binaries for Linux x86_64 and macOS (universal), plus the remaining non-ERiC work: HTTPS download and zip extraction for `eric install`, XML building and parsing, JSON output.

## Answer

**Rust, Linux x86_64 only in v1, with our own runtime binding to ERiC and Otto.** ADR: [Rust, with ERiC loaded at runtime through our own binding](../../../docs/adr/0002-rust-with-runtime-loaded-eric.md).

1. **Language: Rust.** Not provisional: Rust has no runtime of its own (garbage collector, scheduler, signal handlers) to clash with ERiC, and each CLI call is its own short-lived process, the same isolation haben gets from its worker process.
2. **Platform: Linux x86_64 only in v1.** This narrows point 3 of [ERiC for the Kontoabfrage](07-eric-decision.md). macOS is a later effort. Until then, keep the door open: platform-specific details (library file names `.so`/`.dylib`, ERiC package name `Linux-x86_64`/`Darwin-universal`, default data directory) live in one platform table; CI builds and tests Linux only; docs and `eric install` claim nothing beyond Linux x86_64.
3. **Binding: our own `libloading` binding**, with hand-written `extern "C"` function-pointer types for the ~17 ERiC and Otto functions and `#[repr(C)]` copies of `eric_druck_parameter_t` and `eric_verschluesselungs_parameter_t`. Not `eric-bindings`/`eric-sdk` (checked at 0.7.1/0.7.2): they link `libericapi` at build time, so the binary would not start without ERiC and `eric install` could never run on a fresh machine; `ERIC_VERSION` fixes one exact ERiC version per build (newest upstream is 44.2.4.0, ELSTER ships 44.3.6.0); and there is no Otto binding. Later, outside this map: offer eric-rs a runtime-loading mode upstream and switch if accepted.
4. **Safe wrapper:** one `eric` module in a single crate. It owns all `unsafe`, gives every handle (`EricRueckgabepuffer`, certificate handle, `OttoInstanz`, Otto buffer) a `Drop`, and sits behind a trait so commands can be tested against a fake. Split into its own crate only if reuse is wanted.
5. **One ERiC session per process:** `EricInitialisiere` once, all ERiC and Otto calls on the main thread, `EricBeende` before exit. Single-thread API, no `EricMt*`, no worker process, no threads.
6. **Signatures:** only the ERiC ≥ 44 shape of `EricBearbeiteVorgang` (no transfer handle). `EricVersion()` is checked before any other call. A new major adds a variant chosen after that check.
7. **Build target:** `x86_64-unknown-linux-gnu`, not static musl (a static musl binary cannot `dlopen` ERiC's glibc-linked libraries). The glibc baseline must be no newer than ERiC's own; the spike measures it.
8. **Blocking, no async runtime:** ERiC calls block anyway; the `eric install` download uses `ureq` with `rustls` (no system OpenSSL).
9. **Baseline crates and toolchain:** `clap` (derive), `serde` + `serde_json`, `quick-xml` with serde, `zip`, `sha2`, `directories`, `thiserror`; edition 2024; stable toolchain pinned in `rust-toolchain.toml`. Smaller library choices are left to the build.
