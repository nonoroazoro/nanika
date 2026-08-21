use nanika_core::{PRODUCT_NAME, PROJECT_IDENTITY};

#[test]
fn exposes_current_project_identity() {
    assert_eq!(PRODUCT_NAME, "Nanika");
    assert_eq!(PROJECT_IDENTITY.bundle_id, "com.nanika.nanika");
}
