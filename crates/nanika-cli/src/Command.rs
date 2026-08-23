use std::path::PathBuf;

/// Explicit local extension management operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Install(PathBuf),
    Update(PathBuf),
    Enable(String),
    Disable(String),
    Remove(String),
    Diagnostics(PathBuf),
}

impl Command {
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments.into_iter();
        let operation = arguments.next().ok_or_else(usage)?;
        let value = arguments.next().ok_or_else(usage)?;
        if arguments.next().is_some() {
            return Err(usage());
        }
        match operation.as_str() {
            "install" => Ok(Self::Install(PathBuf::from(value))),
            "update" => Ok(Self::Update(PathBuf::from(value))),
            "enable" => Ok(Self::Enable(value)),
            "disable" => Ok(Self::Disable(value)),
            "remove" => Ok(Self::Remove(value)),
            "diagnostics" => Ok(Self::Diagnostics(PathBuf::from(value))),
            _ => Err(usage()),
        }
    }
}

fn usage() -> String {
    "Usage: nanika-cli <install|update|enable|disable|remove|diagnostics> <value>".to_owned()
}

#[cfg(test)]
mod tests {
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
}
