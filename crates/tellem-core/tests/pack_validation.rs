use tellem_core::{Engine, Pack};

#[test]
fn empty_patterns_and_swap_keys_return_a_pack_error() {
    for rule in [
        "kind = 'phrase'\npatterns = ['']",
        "kind = 'word'\npatterns = ['']",
        "kind = 'word'\nswaps = { '' = 'replacement' }",
        "kind = 'regex'\npattern = ''",
    ] {
        let pack = Pack::parse(&format!(
            "[meta]\nname = 'synthetic'\nversion = '0.0.1'\n\
             [[rules]]\nid = 'X001'\nname = 'synthetic'\nrationale = 'synthetic'\n{rule}"
        ))
        .unwrap();
        let error = Engine::from_packs(&[pack]).err().expect("pack accepted");
        assert!(error.to_string().contains("rule X001"));
        assert!(error.to_string().contains("must not be empty"));
    }
}

#[test]
fn shape_rules_require_a_detector_and_cannot_claim_a_fix() {
    for rule in [
        "kind = 'shape'",
        "kind = 'shape'\nshape = 'relabel-pairing'\nreplace = ''",
        "kind = 'shape'\nshape = 'ordinal-drumbeat'\nswaps = { asks = 'says' }",
    ] {
        let pack = Pack::parse(&format!(
            "[meta]\nname = 'synthetic'\nversion = '0.0.1'\n\
             [[rules]]\nid = 'X001'\nname = 'synthetic'\nrationale = 'synthetic'\n{rule}"
        ))
        .unwrap();
        let error = Engine::from_packs(&[pack]).err().expect("pack accepted");
        assert!(error.to_string().contains("rule X001"));
    }
}
