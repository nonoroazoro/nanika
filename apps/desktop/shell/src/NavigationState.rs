use nanika_protocol::{NavigationEffect, View, ViewEvent};

use crate::{ExtensionViewSnapshot, NavigationSnapshot};

pub(crate) const MAX_NAVIGATION_DEPTH: usize = 32;

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
            current: Some(self.stack.last().cloned()),
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

    /// Queued input addresses stable item/action IDs in the current route. Its
    /// captured revision can precede completed selections or invalidations.
    /// A menu instead authorizes the exact snapshot from which it was opened.
    pub(crate) fn authorize_input(
        &self,
        request: &crate::ViewEventRequest,
        menu_revision: Option<u64>,
    ) -> Result<&ExtensionViewSnapshot, String> {
        let route = self.authorize_route(request.route_id)?;
        if menu_revision.is_some_and(|revision| revision != route.revision) {
            return Err("The view changed. Reopen the menu.".to_owned());
        }
        if request.revision > route.revision {
            return Err("The extension view revision is ahead of the current state.".to_owned());
        }
        if let crate::ViewOperation::Event { event } = &request.operation {
            // Confirmation applies to what the user reviewed, never a newer snapshot.
            if matches!(
                event,
                ViewEvent::ActionInvoked {
                    invocation: nanika_protocol::ActionInvocation::Confirmed,
                    ..
                }
            ) && request.revision != route.revision
            {
                return Err("The view changed. Confirm the action again.".to_owned());
            }
            authorize_view_event(&route.view, event)?;
        }
        Ok(route)
    }

    pub(crate) fn finish(&mut self, result: Result<(), String>) {
        self.busy = false;
        self.error = result.err();
        self.revision += 1;
    }

    pub(crate) fn apply(
        &mut self,
        extension_id: &str,
        instance_id: u64,
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
                if self.stack.len() >= MAX_NAVIGATION_DEPTH {
                    return Err(format!(
                        "Navigation is limited to {MAX_NAVIGATION_DEPTH} views. Go back before opening another view."
                    ));
                }
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
                    instance_id,
                    generation,
                    view_id,
                    revision,
                    view: std::sync::Arc::from(view),
                });
            }
        }
        Ok(())
    }
}

pub(crate) fn authorize_view_event(view: &View, event: &ViewEvent) -> Result<(), String> {
    let valid = match (view, event) {
        (_, ViewEvent::Resumed) => true,
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
                invocation,
            },
        ) => list
            .sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| {
                &item.id == item_id
                    && item.actions.iter().any(|action| {
                        &action.id == action_id && action.allows_invocation(*invocation)
                    })
            }),
        (
            View::Detail { detail },
            ViewEvent::ActionInvoked {
                item_id: None,
                action_id,
                invocation,
            },
        ) => detail
            .actions
            .iter()
            .any(|action| &action.id == action_id && action.allows_invocation(*invocation)),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err("The requested interaction is not available in this view.".to_owned())
    }
}
