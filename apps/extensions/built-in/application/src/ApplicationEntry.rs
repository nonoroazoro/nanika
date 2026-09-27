use nanika_protocol::{Candidate, CandidateKind, LaunchDescriptor};

use crate::{ApplicationError, RUN_ACTION_ID};

/// Persisted application metadata plus transient icon extraction input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationEntry {
    _data: std::sync::Arc<crate::ApplicationEntryData>,
    // None is unprepared; an explicit Empty records a completed cache failure.
    // Share presentation across catalog clones without changing persisted extraction inputs.
    pub(crate) _icon: Option<std::sync::Arc<nanika_protocol::IconSource>>,
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
            _icon: None,
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
            icon: Some(
                self._icon
                    .as_deref()
                    .cloned()
                    .unwrap_or(nanika_protocol::IconSource::Empty),
            ),
        }
    }

    pub(crate) fn same_icon_source(&self, other: &Self) -> bool {
        self.icon_key == other.icon_key && self.icon_source == other.icon_source
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
        if self.launch_kind == "windows-packaged" {
            return Ok(LaunchDescriptor::WindowsPackagedApplication {
                app_user_model_id: self.target_path.clone(),
            });
        }
        Err(ApplicationError::Configuration(format!(
            "unsupported application kind: {}",
            self.launch_kind
        )))
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
                icon: None,
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
                    icon: None,
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
