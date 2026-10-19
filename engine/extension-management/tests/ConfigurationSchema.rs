use std::collections::BTreeMap;

use nanika_extension_package::{
    ConfigurationSchema, ConfigurationValueType, parse_extension_manifest
};

#[test]
fn integer_multiple_of_is_anchored_at_zero() {
    let schema = ConfigurationSchema {
        value_type: ConfigurationValueType::Integer,
        format: None,
        minimum: Some(1),
        maximum: Some(10),
        multiple_of: Some(2),
        max_length: None,
        max_items: None,
        items: None,
        properties: BTreeMap::new(),
        required: Vec::new()
    };

    assert!(schema.validate_value(&serde_json::json!(2)).is_ok());
    assert!(schema.validate_value(&serde_json::json!(1)).is_err());
    assert!(schema.validate_value(&serde_json::json!(3)).is_err());
}

#[test]
fn integer_limit_validates_both_fields_even_when_unlimited() {
    let schema: ConfigurationSchema = serde_json::from_value(serde_json::json!({
        "type": "integerLimit", "minimum": 1, "maximum": 5000
    }))
    .unwrap();
    for unlimited in [false, true] {
        assert!(
            schema
                .validate_value(&serde_json::json!({"unlimited":unlimited,"value":50}))
                .is_ok()
        );
        for value in [0, -1, 5001] {
            assert!(
                schema
                    .validate_value(&serde_json::json!({"unlimited":unlimited,"value":value}))
                    .is_err()
            );
        }
    }
    for value in [
        serde_json::Value::Null,
        serde_json::json!(50),
        serde_json::json!({"unlimited":true}),
        serde_json::json!({"unlimited":"true","value":50}),
        serde_json::json!({"unlimited":true,"value":50,"extra":1})
    ] {
        assert!(schema.validate_value(&value).is_err());
    }
}

#[test]
fn integer_bounds_and_step_are_optional() {
    for kind in ["integer", "integerLimit"] {
        let schema: ConfigurationSchema =
            serde_json::from_value(serde_json::json!({"type":kind})).unwrap();
        for value in [-9007199254740991_i64, 0, 9007199254740991_i64] {
            let value = if kind == "integerLimit" {
                serde_json::json!({"unlimited":true,"value":value})
            } else {
                serde_json::json!(value)
            };
            assert!(schema.validate_value(&value).is_ok());
        }
    }
}

#[test]
fn integer_values_must_fit_the_frontend_safe_integer_range() {
    let schema = ConfigurationSchema {
        value_type: ConfigurationValueType::Integer,
        format: None,
        minimum: Some(i64::MIN),
        maximum: Some(i64::MAX),
        multiple_of: Some(1),
        max_length: None,
        max_items: None,
        items: None,
        properties: BTreeMap::new(),
        required: Vec::new()
    };

    assert!(
        schema
            .validate_value(&serde_json::json!(9_007_199_254_740_991_i64))
            .is_ok()
    );
    assert!(
        schema
            .validate_value(&serde_json::json!(9_007_199_254_740_992_i64))
            .is_err()
    );
    assert!(
        schema
            .validate_value(&serde_json::json!(-9_007_199_254_740_992_i64))
            .is_err()
    );
}

#[test]
fn manifest_rejects_integer_constraints_outside_the_frontend_safe_range() {
    let mut manifest = serde_json::json!({
        "format": "nanika-extension",
        "name": "Test Extension",
        "icon": "assets/icon.png",
        "manifestVersion": 1,
        "id": "test.extension",
        "version": "0.1.0",
        "hostApi": "^0.1",
        "targets": {
            "x86_64-pc-windows-msvc": {
                "entrypoint": "bin/x86_64-pc-windows-msvc/test.exe"
            }
        },
        "runtime": { "protocol": "nanika", "protocolVersion": 1 },
        "contributes": {
            "configuration": {
                "title": "Test",
                "properties": {
                    "test.limit": {
                        "type": "integer",
                        "title": "Limit",
                        "persistence": "beforeApply", "default": 1,
                        "minimum": 1,
                        "maximum": 9007199254740991_i64,
                        "multipleOf": 1
                    }
                }
            }
        }
    });
    assert!(parse_extension_manifest(&manifest.to_string()).is_ok());

    for (field, value) in [
        ("minimum", serde_json::json!(-9_007_199_254_740_992_i64)),
        ("maximum", serde_json::json!(9_007_199_254_740_992_i64)),
        ("multipleOf", serde_json::json!(9_007_199_254_740_992_u64))
    ] {
        manifest["contributes"]["configuration"]["properties"]["test.limit"][field] = value;
        assert!(parse_extension_manifest(&manifest.to_string()).is_err());
        manifest["contributes"]["configuration"]["properties"]["test.limit"][field] = match field {
            "minimum" | "multipleOf" => serde_json::json!(1),
            "maximum" => serde_json::json!(9_007_199_254_740_991_i64),
            _ => unreachable!()
        };
    }
}
