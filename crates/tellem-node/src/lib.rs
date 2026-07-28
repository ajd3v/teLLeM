//! Node bindings: the platform's deai pass. `deAi` mirrors the TS signature
//! so gainful's deai.ts becomes a one-line consumer once parity is accepted.

#[macro_use]
extern crate napi_derive;

use std::sync::OnceLock;
use tellem_core::{Engine, Pack};

static ENGINE: OnceLock<Engine> = OnceLock::new();

fn engine() -> &'static Engine {
    ENGINE.get_or_init(|| {
        Engine::from_packs(&[Pack::parse(tellem_core::BASE_PACK).unwrap()]).unwrap()
    })
}

/// Load an extra pack on top of base (rule ids override). Consumers with their
/// own voice rules call this once at import time, before any deAi call.
#[napi(js_name = "setPack")]
pub fn set_pack(toml_src: String) -> napi::Result<()> {
    let reason = |e: tellem_core::Error| napi::Error::from_reason(e.to_string());
    let packs = [
        Pack::parse(tellem_core::BASE_PACK).map_err(reason)?,
        Pack::parse(&toml_src).map_err(reason)?,
    ];
    let engine = Engine::from_packs(&packs).map_err(reason)?;
    ENGINE.set(engine).map_err(|_| {
        napi::Error::from_reason("setPack must be called once, before the first deAi or lintJson")
    })
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
