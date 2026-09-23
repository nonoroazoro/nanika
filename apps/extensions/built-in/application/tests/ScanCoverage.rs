use std::path::Path;

use crate::scan_coverage::ScanCoverage;

#[test]
fn failures_protect_only_their_path_and_descendants() {
    let mut coverage = ScanCoverage::new(["/apps".to_owned()].into_iter(), true);
    coverage.failed(Path::new("/apps/blocked"));
    assert!(!coverage.replaces("/apps/blocked"));
    assert!(!coverage.replaces("/apps/blocked/old.exe"));
    assert!(coverage.replaces("/apps/blocked-other/old.exe"));
    assert!(coverage.replaces("/apps/deleted.exe"));
    assert!(coverage.replaces("/removed-root/old.exe"));
}

#[test]
fn unresolved_roots_limit_cleanup_to_known_paths() {
    let coverage = ScanCoverage::new(["/apps/".to_owned()].into_iter(), false);
    assert!(coverage.replaces("/apps/deleted.exe"));
    assert!(!coverage.replaces("/apps-other/old.exe"));
    assert!(!coverage.replaces("/unknown-root/old.exe"));
}
