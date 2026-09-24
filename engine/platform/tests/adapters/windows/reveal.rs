use super::_shell_path;
use std::path::Path;

#[test]
fn canonical_paths_use_shell_compatible_spelling() {
    for (input, expected) in [
        (r"\\?\C:\Apps\An App\app.exe", r"C:\Apps\An App\app.exe"),
        (r"\\?\UNC\server\share\文件.exe", r"\\server\share\文件.exe"),
        (r"C:\Apps\app.exe", r"C:\Apps\app.exe"),
    ] {
        assert_eq!(_shell_path(Path::new(input)).unwrap(), Path::new(expected));
    }
    assert!(_shell_path(Path::new("relative.exe")).is_err());
    assert!(_shell_path(Path::new(r"\\.\PhysicalDrive0")).is_err());
}
