use std::collections::{BTreeMap, VecDeque};
use tauri::ipc::Channel;

use crate::{SettingsApplicationUpdate, SettingsEvent, SettingsSaveResult};

/// Retains operation facts separately from bounded, replaceable progress delivery.
#[derive(Default)]
pub(crate) struct SettingsApplications {
    pub(crate) latest: BTreeMap<String, SettingsApplicationUpdate>,
    pub(crate) updates: Option<Channel<SettingsEvent>>,
    pending: VecDeque<SettingsApplicationUpdate>,
    in_flight: Option<u64>,
    next_delivery_id: u64,
}

impl SettingsApplications {
    pub(crate) fn subscribe(&mut self, channel: Channel<SettingsEvent>) {
        self.updates = Some(channel);
        self.in_flight = None;
        self.pending.clear();
        // Never reuse receipt IDs across WebViews. Their delayed acknowledgements
        // must not release a newer session's slot.
    }

    pub(crate) fn record(
        &mut self,
        update: SettingsApplicationUpdate,
    ) -> Option<(Channel<SettingsEvent>, SettingsEvent)> {
        if self
            .latest
            .get(&update.extension_id)
            .is_some_and(|current| {
                current.request_id > update.request_id
                    || (current.request_id == update.request_id
                        && (!matches!(current.result, SettingsSaveResult::Running { .. })
                            || matches!(
                                (&current.result, &update.result),
                                (
                                    SettingsSaveResult::Running { progress: Some(_) },
                                    SettingsSaveResult::Running { progress: None }
                                )
                            )))
            })
        {
            return None;
        }
        self.latest
            .insert(update.extension_id.clone(), update.clone());
        let channel = self.updates.clone()?;
        if matches!(
            update.result,
            SettingsSaveResult::Running { progress: Some(_) }
        ) {
            if let Some(pending) = self
                .pending
                .iter_mut()
                .find(|pending| pending.extension_id == update.extension_id)
            {
                *pending = update;
            } else {
                self.pending.push_back(update);
            }
            self._next_progress()
        } else {
            // Terminal results bypass progress receipts and cannot be replaced by
            // an undelivered presentation update.
            self.pending
                .retain(|pending| pending.extension_id != update.extension_id);
            Some((
                channel,
                SettingsEvent::Application {
                    update,
                    progress_delivery_id: None,
                },
            ))
        }
    }

    pub(crate) fn acknowledge(
        &mut self,
        delivery_id: u64,
    ) -> Option<(Channel<SettingsEvent>, SettingsEvent)> {
        if self.in_flight != Some(delivery_id) {
            return None;
        }
        self.in_flight = None;
        self._next_progress()
    }

    pub(crate) fn disconnect(&mut self, channel_id: u32) {
        if self
            .updates
            .as_ref()
            .is_some_and(|channel| channel.id() == channel_id)
        {
            self.updates = None;
            self.in_flight = None;
            self.pending.clear();
        }
    }

    fn _next_progress(&mut self) -> Option<(Channel<SettingsEvent>, SettingsEvent)> {
        if self.in_flight.is_some() {
            return None;
        }
        let channel = self.updates.clone()?;
        let update = self.pending.pop_front()?;
        self.next_delivery_id += 1;
        self.in_flight = Some(self.next_delivery_id);
        Some((
            channel,
            SettingsEvent::Application {
                update,
                progress_delivery_id: self.in_flight,
            },
        ))
    }
}
