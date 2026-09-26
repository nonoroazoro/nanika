use nanika_protocol::{Candidate, CandidateKind, IconReference, LaunchArguments, LaunchDescriptor};

use crate::{ApplicationArguments, ApplicationError, RUN_ACTION_ID};

/// Persisted application metadata plus transient icon extraction input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationEntry {
    _data: std::sync::Arc<crate::ApplicationEntryData>,
    pub(crate) _icon_ready: bool,
}

impl std::ops::Deref for ApplicationEntry {
    type Target = crate::ApplicationEntryData;
    fn deref(&self) -> &Self::Target {
        &self._data
    }
}

impl std::ops::DerefMut for ApplicationEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        std::sync::Arc::make_mut(&mut self._data)
    }
}

impl ApplicationEntry {
    pub fn new(data: crate::ApplicationEntryData) -> Self {
        Self {
            _data: std::sync::Arc::new(data),
            _icon_ready: false,
        }
    }

    pub fn candidate(&self) -> Candidate {
        let aliases = self
            .normalized_tokens
            .lines()
            .filter(|alias| *alias != self.normalized_name)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        Candidate {
            kind: CandidateKind::Action,
            entry_id: self.entry_id.clone(),
            title: self.display_name.clone(),
            subtitle: Some(nanika_protocol::CandidateSubtitle::Label(
                "Application".to_owned(),
            )),
            action_id: RUN_ACTION_ID.to_owned(),
            actions: self.actions(),
            aliases,
            icon: self
                ._icon_ready
                .then(|| IconReference::new(&self.icon_key).ok())
                .flatten()
                .map(nanika_protocol::IconSource::Cache),
        }
    }

    pub fn launch_descriptor(&self) -> Result<LaunchDescriptor, ApplicationError> {
        if matches!(
            self.launch_kind.as_str(),
            "windows-shell-link" | "executable"
        ) {
            return Ok(LaunchDescriptor::WindowsApplication {
                path: self.target_path.clone(),
            });
        }
        if self.launch_kind == "macos-bundle" {
            return Ok(LaunchDescriptor::MacApplication {
                bundle_path: self.target_path.clone(),
            });
        }
        let arguments = match serde_json::from_str::<ApplicationArguments>(&self.arguments_json)? {
            ApplicationArguments::Structured { values } => LaunchArguments::Structured { values },
            ApplicationArguments::WindowsRaw { value } => LaunchArguments::WindowsRaw { value },
        };
        Ok(LaunchDescriptor::Program {
            program: self.target_path.clone(),
            arguments,
            working_directory: None,
        })
    }

    pub fn actions(&self) -> Vec<nanika_protocol::Action> {
        let mut actions = vec![nanika_protocol::Action::primary(RUN_ACTION_ID, "Open")];
        if self.launch_kind != "windows-packaged" {
            let title = match self.launch_kind.as_str() {
                "windows-shell-link" => "Open shortcut location",
                "macos-bundle" => "Show in Finder",
                _ => "Open file location",
            };
            actions.push(nanika_protocol::Action {
                id: "application.reveal".to_owned(),
                title: title.to_owned(),
                allow_default_execution: false,
                style: nanika_protocol::ActionStyle::Secondary,
                enabled: true,
                group: Some("location".to_owned()),
                confirmation_title: None,
            });
            if self.launch_kind == "windows-shell-link" {
                actions.push(nanika_protocol::Action {
                    id: "application.revealTarget".to_owned(),
                    title: "Open target location".to_owned(),
                    allow_default_execution: false,
                    style: nanika_protocol::ActionStyle::Secondary,
                    enabled: true,
                    group: Some("location".to_owned()),
                    confirmation_title: None,
                });
            }
        }
        actions
    }

    pub fn host_request(
        &self,
        action: &str,
    ) -> Result<nanika_protocol::HostServiceRequest, ApplicationError> {
        match action {
            RUN_ACTION_ID => Ok(nanika_protocol::HostServiceRequest::Launch {
                descriptor: self.launch_descriptor()?,
            }),
            "application.reveal" if self.launch_kind != "windows-packaged" => {
                Ok(nanika_protocol::HostServiceRequest::RevealPath {
                    path: self.target_path.clone(),
                })
            }
            "application.revealTarget" if self.launch_kind == "windows-shell-link" => {
                Ok(nanika_protocol::HostServiceRequest::RevealPath {
                    path: crate::platform::shortcut_target(std::path::Path::new(
                        &self.target_path,
                    ))?,
                })
            }
            _ => Err(ApplicationError::Configuration(
                "application action is unavailable".to_owned(),
            )),
        }
    }
}
