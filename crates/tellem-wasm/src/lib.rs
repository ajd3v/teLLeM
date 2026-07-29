//! Browser bindings: base pack embedded, zero setup, client-side only.
// ponytail: base pack only, add a `load_pack(toml)` export when a consumer
// needs private packs in the browser.

use std::sync::OnceLock;
use tellem_core::{Engine, Pack};
use wasm_bindgen::prelude::*;

static CATALOG: OnceLock<tellem_core::mine::Catalog> = OnceLock::new();

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

/// Load a mined catalog (TOML) once, so `who` can score without re-parsing.
#[wasm_bindgen]
pub fn load_catalog(toml_src: &str) -> Result<(), JsValue> {
    let cat: tellem_core::mine::Catalog =
        toml::from_str(toml_src).map_err(|e| JsValue::from_str(&e.to_string()))?;
    CATALOG
        .set(cat)
        .map_err(|_| JsValue::from_str("catalog already loaded"))
}

/// Closed-set attribution as JSON. Refuses below `min_confidence`, and refuses
/// outright when the rejection class ranks first.
#[wasm_bindgen]
pub fn who(text: &str, min_confidence: f32) -> Result<String, JsValue> {
    let cat = CATALOG
        .get()
        .ok_or_else(|| JsValue::from_str("call load_catalog first"))?;
    serde_json::to_string(&cat.who(text, min_confidence, 20))
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
