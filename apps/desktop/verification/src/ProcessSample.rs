#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSample {
    pub pid: u32,
    pub parent: Option<u32>,
    pub name: String,
    pub resident_bytes: u64,
    pub cpu_milliseconds: u64
}
