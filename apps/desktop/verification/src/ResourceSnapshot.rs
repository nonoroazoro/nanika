#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSnapshot {
    pub elapsed_ms: u128,
    pub processes: Vec<super::process_sample::ProcessSample>
}
