mod instance {
    use nanika_platform::{InstanceRole, acquire_instance, signal_activate};

    #[test]
    fn second_launch_signals_the_primary() {
        let identity = format!("com.nanika.test.{}", std::process::id());
        let root = std::env::temp_dir().join(&identity);
        let primary = acquire_instance(&identity, &root).expect("primary should acquire");
        let mut instance = match primary {
            InstanceRole::Primary(instance) => instance,
            InstanceRole::Secondary => panic!("first launch became secondary"),
        };
        let events = instance.take_events().expect("event receiver should exist");

        assert!(matches!(
            acquire_instance(&identity, &root).expect("second launch should acquire role"),
            InstanceRole::Secondary
        ));
        signal_activate(&identity, &root).expect("second launch should signal activation");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        loop {
            let event = events
                .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                .expect("primary should receive activation");
            if event == nanika_platform::PlatformEvent::Open {
                break;
            }
        }
        drop(events);
        drop(instance);

        let restarted = acquire_instance(&identity, &root).expect("restarted host should acquire");
        let InstanceRole::Primary(restarted) = restarted else {
            panic!("restarted host became secondary");
        };
        drop(restarted);

        let _ = std::fs::remove_file(root.join("nanika.instance.lock"));
        let _ = std::fs::remove_dir(root);
    }
}

mod filesystem {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use nanika_platform::{
        atomic_replace, companion_executable, make_executable, open_regular_file, target_triple,
    };

    #[test]
    fn replacement_creates_and_replaces_the_destination() {
        let root = test_root("replace");
        let source = root.join("prepared-file");
        let target = root.join("destination");
        for content in ["first", "replacement"] {
            fs::write(&source, content).expect("prepare replacement");
            atomic_replace(&source, &target).expect("replace destination");
            assert_eq!(
                fs::read_to_string(&target).expect("read destination"),
                content
            );
            assert!(!source.exists());
        }
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn missing_replacement_preserves_the_destination() {
        let root = test_root("missing");
        let source = root.join("missing-file");
        let target = root.join("destination");
        fs::write(&target, "original").expect("write destination");
        let error = atomic_replace(&source, &target).expect_err("missing replacement must fail");
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(
            fs::read_to_string(&target).expect("read destination"),
            "original"
        );
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn failed_replacement_preserves_both_source_and_destination() {
        let root = test_root("blocked");
        let source = root.join("prepared-file");
        let target = root.join("occupied-directory");
        fs::write(&source, "prepared").expect("prepare replacement");
        fs::create_dir(&target).expect("create occupied destination");
        fs::write(target.join("entry"), "original").expect("write original entry");
        atomic_replace(&source, &target)
            .expect_err("a file must not replace an occupied directory");
        assert_eq!(
            fs::read_to_string(&source).expect("read source"),
            "prepared"
        );
        assert_eq!(
            fs::read_to_string(target.join("entry")).expect("read original entry"),
            "original"
        );
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn executable_preparation_preserves_contents_and_uses_the_artifact_conventions() {
        let root = test_root("executable");
        let program = companion_executable(&root.join("host"), "extension");
        fs::write(&program, "executable contents").expect("write executable");
        make_executable(&program).expect("prepare executable");
        assert_eq!(
            fs::read_to_string(&program).expect("read executable"),
            "executable contents"
        );
        assert_eq!(program.parent(), Some(root.as_path()));
        #[cfg(target_os = "windows")]
        {
            assert_eq!(program.file_name().expect("file name"), "extension.exe");
            assert_eq!(target_triple(), "x86_64-pc-windows-msvc");
        }
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::PermissionsExt;

            assert_eq!(program.file_name().expect("file name"), "extension");
            assert_eq!(
                fs::metadata(&program)
                    .expect("executable metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o755
            );
            assert!(matches!(
                target_triple(),
                "aarch64-apple-darwin" | "x86_64-apple-darwin"
            ));
        }
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn regular_file_open_preserves_contents_and_rejects_a_directory() {
        use std::io::Read;

        let root = test_root("regular-file");
        let path = root.join("diagnostic.log");
        fs::write(&path, "diagnostic contents").expect("write diagnostic");
        let mut file = open_regular_file(&path).expect("open regular diagnostic");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("read diagnostic");
        assert_eq!(contents, "diagnostic contents");
        assert!(open_regular_file(&root).is_err());
        drop(file);
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn regular_file_open_rejects_a_symbolic_link() {
        let root = test_root("symlink");
        let path = root.join("diagnostic.log");
        let link = root.join("linked.log");
        fs::write(&path, "diagnostic contents").expect("write diagnostic");
        std::os::unix::fs::symlink(&path, &link).expect("create symbolic link");
        let error = open_regular_file(&link).expect_err("symbolic link must be rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        fs::remove_dir_all(root).expect("remove test directory");
    }

    fn test_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("valid clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "nanika-platform-filesystem-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("create unique test directory");
        root
    }
}
