use std::path::PathBuf;

use nanika_protocol::{Candidate, CandidateKind, IconReference, LaunchArguments, LaunchDescriptor};
use nanika_text_search::{RomanizedReading, romanized_readings};

use crate::{ApplicationArguments, ApplicationError, RUN_ACTION_ID};

/// Persisted application metadata plus transient icon extraction input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationEntry {
    pub entry_id: String,
    pub source_key: String,
    pub display_name: String,
    pub normalized_name: String,
    pub normalized_tokens: String,
    /// Prepared search spellings; derived from the original names, never persisted.
    pub search_readings: Vec<RomanizedReading>,
    pub launch_kind: String,
    /// Native activation path, preserving the original spelling of Shell Links.
    pub target_path: String,
    pub working_directory: Option<String>,
    pub arguments_json: String,
    pub bundle_id: Option<String>,
    pub icon_key: String,
    pub(crate) icon_source: Option<PathBuf>,
    pub(crate) icon_index: i32,
    pub(crate) priority: usize,
}

impl ApplicationEntry {
    pub(crate) fn prepare_search_readings(&mut self) {
        let mut readings = romanized_readings(&self.display_name);
        for alias in self
            .normalized_tokens
            .lines()
            .filter(|alias| *alias != self.normalized_name)
        {
            for reading in romanized_readings(alias) {
                if !readings.contains(&reading) {
                    readings.push(reading);
                }
            }
        }
        self.search_readings = readings;
    }

    pub fn candidate(&self) -> Candidate {
        let mut aliases = self
            .normalized_tokens
            .lines()
            .filter(|alias| *alias != self.normalized_name)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        for reading in &self.search_readings {
            for alias in [&reading.full, &reading.initials] {
                if alias != &self.normalized_name && !aliases.iter().any(|item| item == alias) {
                    aliases.push(alias.clone());
                }
            }
        }
        Candidate {
            kind: CandidateKind::Action,
            entry_id: self.entry_id.clone(),
            title: self.display_name.clone(),
            subtitle: Some("Application".to_owned()),
            action_id: RUN_ACTION_ID.to_owned(),
            actions: self.actions(),
            aliases,
            icon: IconReference::new(&self.icon_key)
                .ok()
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
            working_directory: self.working_directory.clone(),
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
