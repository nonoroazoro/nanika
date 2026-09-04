use nanika_extension_calculator::CalculatorEngine;

#[test]
fn calculator_returns_deterministic_live_results() {
    let engine = CalculatorEngine::new();
    let (candidate, result) = engine.evaluate("2 + 3 * 4").expect("calculation result");
    assert_eq!(result, "14");
    assert_eq!(candidate.title, "= 14");
    assert_eq!(candidate.aliases, ["2 + 3 * 4"]);
}

#[test]
fn invalid_or_identity_inputs_do_not_contribute_results() {
    let engine = CalculatorEngine::new();
    assert!(engine.evaluate("").is_none());
    assert!(engine.evaluate("hello").is_none());
    assert!(engine.evaluate("aa").is_none());
    assert!(engine.evaluate("mA").is_none());
    assert!(engine.evaluate("Books").is_none());
}

#[test]
fn explicit_word_operators_enable_unit_conversions() {
    let engine = CalculatorEngine::new();
    let (_, result) = engine
        .evaluate("15 km to mi")
        .expect("conversion should produce a result");

    assert!(result.contains("mi"));
}
