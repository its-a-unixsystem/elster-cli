# Rust, with ERiC loaded at runtime through our own binding

KIECLI is written in Rust and binds `libericapi` and `libotto` through its own hand-written `libloading` declarations, not through the existing `eric-bindings`/`eric-sdk` crates. Once ERiC carries all ELSTER traffic, the language choice no longer turns on transport libraries, where Go was ahead; Rust has no runtime of its own that could clash with ERiC in the same process, and v1 targets Linux x86_64 only, which removes Rust's main drawback, the macOS cross-build toolchain. The binding is about 17 functions and two structs; owning it keeps the binary startable without ERiC (so `kiecli eric install` works on a fresh machine) and lets one release check and accept several ERiC versions at runtime.

## Considered Options

- **Go with `purego`**: pure cross-builds to macOS and a standard library covering download, zip, XML and JSON, but ERiC (C++) would run inside the Go runtime with untested signal and thread interplay.
- **`eric-sdk`/`eric-bindings`**: maintained and less code of our own, but they link `libericapi` at build time (the binary refuses to start without ERiC), fix one exact ERiC version per build via `ERIC_VERSION`, and have no Otto binding.

## Consequences

- Supporting a new ERiC major means adding its signatures behind the `EricVersion()` check, not rebuilding against new headers.
- If eric-rs ever gains a runtime-loading mode, switching to it and deleting our binding is worth reconsidering.
- macOS is a later effort: platform-specific details stay in one place so it can be added without restructuring.
