#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PackageOperation {
    Install,
    Update,
}
