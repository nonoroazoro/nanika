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
}
