use std::collections::BTreeMap;

use nanika_extension_package::{ConfigurationSchema, ConfigurationValueType};

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
        required: Vec::new(),
    };

    assert!(schema.validate_value(&serde_json::json!(2)).is_ok());
    assert!(schema.validate_value(&serde_json::json!(1)).is_err());
    assert!(schema.validate_value(&serde_json::json!(3)).is_err());
}
