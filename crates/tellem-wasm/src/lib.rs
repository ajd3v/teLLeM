//! Browser bindings: base pack embedded, zero setup, client-side only.
// ponytail: base pack only, add a `load_pack(toml)` export when a consumer
// needs private packs in the browser.

use std::sync::OnceLock;
use tellem_core::{Engine, Pack};
use wasm_bindgen::prelude::*;

fn engine() -> &'static Engine {
    static E: OnceLock<Engine> = OnceLock::new();
    E.get_or_init(|| Engine::from_packs(&[Pack::parse(tellem_core::BASE_PACK).unwrap()]).unwrap())
}

/// Full lint report as JSON (findings with receipts, words, score, band).
#[wasm_bindgen]
pub fn lint_json(text: &str) -> String {
    serde_json::to_string(&engine().lint(text)).unwrap()
}

/// Deterministic de-AI rewrite.
#[wasm_bindgen]
pub fn fix(text: &str) -> String {
    engine().fix(text)
}
