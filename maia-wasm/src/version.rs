//! Version information about maia-wasm.

use wasm_bindgen::prelude::*;

/// Gives the maia-wasm version as a `String`.
#[wasm_bindgen]
pub fn maia_wasm_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Gives the version of the git repository as a `String`.
#[wasm_bindgen]
pub fn maia_wasm_git_version() -> String {
    git_version::git_version!(fallback = "unknown").to_string()
}
