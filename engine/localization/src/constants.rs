// Admission bounds are independent of each destination field's stricter output limit.
pub const MAX_RESOURCE_BYTES: usize = 256 * 1024;
pub const MAX_LOCALE_BYTES: usize = 1024 * 1024;
pub const MAX_LOCALE_RESOURCES: usize = 16;
pub const MAX_RESOURCE_ENTRIES: usize = 2_048;
pub const MAX_REFERENCE_DEPTH: usize = 32;
pub const MAX_EXPANDED_BYTES: usize = 64 * 1024;
// Five shipping extension owners fit without churn. Mounted surfaces retain their own handles.
pub const MAX_CACHED_RESOURCE_SETS: usize = 8;
pub const MAX_RESOURCE_LOADS: usize = 4;
