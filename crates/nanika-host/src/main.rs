//! Nanika host entry point.

fn main() {
    let _identity = nanika_core::PROJECT_IDENTITY;
    let _platform = nanika_platform::target_platform();
    let _protocol = nanika_protocol::PROTOCOL_NAME;
    let _paths = nanika_storage::NanikaPaths::discover();
    let _config = nanika_config::CONFIG_FORMAT_VERSION;
}
