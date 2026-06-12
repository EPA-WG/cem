//! Host-facing API adapters.
//!
//! Public Rust callers consume `cem_ml` types directly. The submodules
//! here adapt those Rust types to specific host environments — today
//! that means JavaScript via WebAssembly. Each adapter is feature- or
//! `cfg`-gated so it costs nothing on platforms that do not need it.
//!
//! AC mapping:
//!
//! - [`wasm`] exposes the AC-O-1 observer surface (`onParseEvent`,
//!   `onValidate`, `onTransform`) to JS callers per AC-C-1 (browser /
//!   Node parity with the Rust surface).

#[cfg(target_arch = "wasm32")]
pub mod wasm;
