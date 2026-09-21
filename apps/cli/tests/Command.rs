use super::Command;

#[test]
fn parser_keeps_package_paths_and_extension_ids_typed() {
    assert_eq!(
        Command::parse(["install".to_owned(), "example.nanika".to_owned()]),
        Ok(Command::Install("example.nanika".into()))
    );
    assert_eq!(
        Command::parse(["update".to_owned(), "example.nanika".to_owned()]),
        Ok(Command::Update("example.nanika".into()))
    );
    assert_eq!(
        Command::parse(["disable".to_owned(), "com.example.tool".to_owned()]),
        Ok(Command::Disable("com.example.tool".to_owned()))
    );
}
