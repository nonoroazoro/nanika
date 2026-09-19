use std::collections::BTreeMap;

use nanika_extension_package::{
    ConfigurationSchema, ConfigurationValueType, parse_extension_manifest,
};

#[test]
fn integer_multiple_of_is_anchored_at_zero() {
    let schema = ConfigurationSchema {
        value_type: ConfigurationValueType::Integer,
        allow_unlimited: false,
        format: None,
        minimum: Some(1),
        maximum: Some(10),
        multiple_of: Some(2),
        max_length: None,
        max_items: None,
        items: None,
        properties: BTreeMap::new(),
        required: Vec::new(),
    };

    assert!(schema.validate_value(&serde_json::json!(2)).is_ok());
    assert!(schema.validate_value(&serde_json::json!(1)).is_err());
    assert!(schema.validate_value(&serde_json::json!(3)).is_err());
}

#[test]
fn unlimited_is_explicit_and_zero_is_not_a_sentinel() {
    let mut schema: ConfigurationSchema = serde_json::from_value(serde_json::json!({
        "type": "integer", "minimum": 1, "maximum": 5000, "multipleOf": 1,
        "allowUnlimited": true
    }))
    .unwrap();
    assert!(schema.validate_value(&serde_json::Value::Null).is_ok());
    assert!(schema.validate_value(&serde_json::json!(50)).is_ok());
    assert!(schema.validate_value(&serde_json::json!(0)).is_err());
    assert!(schema.validate_value(&serde_json::json!(-1)).is_err());
    schema.allow_unlimited = false;
    assert!(schema.validate_value(&serde_json::Value::Null).is_err());
}

#[test]
fn integer_values_must_fit_the_frontend_safe_integer_range() {
    let schema = ConfigurationSchema {
        value_type: ConfigurationValueType::Integer,
        allow_unlimited: false,
        format: None,
        minimum: Some(i64::MIN),
        maximum: Some(i64::MAX),
        multiple_of: Some(1),
        max_length: None,
        max_items: None,
        items: None,
        properties: BTreeMap::new(),
        required: Vec::new(),
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
        "icon": "extension",
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
                        "default": 1,
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
        ("multipleOf", serde_json::json!(9_007_199_254_740_992_u64)),
    ] {
        manifest["contributes"]["configuration"]["properties"]["test.limit"][field] = value;
        assert!(parse_extension_manifest(&manifest.to_string()).is_err());
        manifest["contributes"]["configuration"]["properties"]["test.limit"][field] = match field {
            "minimum" | "multipleOf" => serde_json::json!(1),
            "maximum" => serde_json::json!(9_007_199_254_740_991_i64),
            _ => unreachable!(),
        };
    }
}
