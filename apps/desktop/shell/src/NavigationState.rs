use nanika_protocol::{NavigationEffect, View, ViewEvent};

use crate::{ExtensionViewSnapshot, NavigationSnapshot};

#[derive(Default)]
pub(crate) struct NavigationState {
    pub(crate) stack: Vec<ExtensionViewSnapshot>,
    pub(crate) revision: u64,
    pub(crate) busy: bool,
    error: Option<String>,
    dismiss_count: u64,
    next_route_id: u64,
}

impl NavigationState {
    pub(crate) fn snapshot(&self) -> NavigationSnapshot {
        NavigationSnapshot {
            revision: self.revision,
            current: self.stack.last().cloned(),
            busy: self.busy,
            error: self.error.clone(),
            dismiss_count: self.dismiss_count,
        }
    }

    pub(crate) fn begin(&mut self) -> Result<(), String> {
        if self.busy {
            return Err("An extension operation is still pending.".to_owned());
        }
        self.busy = true;
        self.error = None;
        self.revision += 1;
        Ok(())
    }

    pub(crate) fn authorize_view(
        &self,
        route_id: u64,
        revision: u64,
    ) -> Result<&ExtensionViewSnapshot, String> {
        let current = self.stack.last().ok_or("No extension view is open.")?;
        if current.route_id != route_id || current.revision != revision {
            return Err("The extension view changed. Use its current state.".to_owned());
        }
        Ok(current)
    }

    pub(crate) fn authorize_route(&self, route_id: u64) -> Result<&ExtensionViewSnapshot, String> {
        self.stack
            .last()
            .filter(|route| route.route_id == route_id)
            .ok_or_else(|| "The extension view changed. Use its current state.".to_owned())
    }

    pub(crate) fn finish(&mut self, result: Result<(), String>) {
        self.busy = false;
        self.error = result.err();
        self.revision += 1;
    }

    pub(crate) fn apply(
        &mut self,
        extension_id: &str,
        generation: u64,
        effect: NavigationEffect,
    ) -> Result<(), String> {
        effect.validate()?;
        match effect {
            NavigationEffect::None => {}
            NavigationEffect::Dismiss => {
                self.dismiss_count += 1;
            }
            NavigationEffect::Pop => {
                self.stack.pop();
            }
            NavigationEffect::Push {
                view_id,
                revision,
                view,
            } => {
                if self
                    .stack
                    .iter()
                    .any(|route| route.extension_id == extension_id && route.view_id == view_id)
                {
                    return Err("The extension tried to reopen an active view id.".to_owned());
                }
                self.next_route_id += 1;
                self.stack.push(ExtensionViewSnapshot {
                    route_id: self.next_route_id,
                    extension_id: extension_id.to_owned(),
                    generation,
                    view_id,
                    revision,
                    view: *view,
                });
            }
        }
        Ok(())
    }
}

pub(crate) fn authorize_view_event(view: &View, event: &ViewEvent) -> Result<(), String> {
    let valid = match (view, event) {
        (View::List { .. }, ViewEvent::SearchChanged { text }) => text.chars().count() <= 4096,
        (View::List { list }, ViewEvent::SelectionChanged { item_id }) => {
            item_id.as_ref().is_none_or(|id| {
                list.sections
                    .iter()
                    .flat_map(|section| &section.items)
                    .any(|item| &item.id == id)
            })
        }
        (View::List { list }, ViewEvent::FilterChanged { filter_id, value }) => {
            list.filter.as_ref().is_some_and(|filter| {
                &filter.id == filter_id
                    && filter.options.iter().any(|option| &option.value == value)
            })
        }
        (View::List { list }, ViewEvent::LoadMore { cursor }) => {
            list.next_cursor.as_ref() == Some(cursor)
        }
        (
            View::List { list },
            ViewEvent::ActionInvoked {
                item_id: Some(item_id),
                action_id,
            },
        ) => list
            .sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| {
                &item.id == item_id && item.actions.iter().any(|action| &action.id == action_id)
            }),
        (
            View::Detail { detail },
            ViewEvent::ActionInvoked {
                item_id: None,
                action_id,
            },
        ) => detail.actions.iter().any(|action| &action.id == action_id),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err("The requested interaction is not available in this view.".to_owned())
    }
}
