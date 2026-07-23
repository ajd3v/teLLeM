//! Node bindings: the platform's deai pass. `deAi` mirrors the TS signature
//! so gainful's deai.ts becomes a one-line consumer once parity is accepted.

#[macro_use]
extern crate napi_derive;

use std::sync::OnceLock;
use tellem_core::{Engine, Pack};

fn engine() -> &'static Engine {
    static E: OnceLock<Engine> = OnceLock::new();
    E.get_or_init(|| Engine::from_packs(&[Pack::parse(tellem_core::BASE_PACK).unwrap()]).unwrap())
}

/// Deterministic de-AI rewrite (drop-in for the TS deAi()).
#[napi(js_name = "deAi")]
pub fn de_ai(text: String) -> String {
    engine().fix(&text)
}

/// Full lint report as JSON (findings with receipts, words, score, band).
#[napi]
pub fn lint_json(text: String) -> String {
    serde_json::to_string(&engine().lint(&text)).unwrap()
}
