use nanika_protocol::{DetailContent, DetailView, View, ViewEvent};

use crate::authorize_view_event;

#[test]
fn resumed_events_are_authorized_for_every_host_rendered_view() {
    let view = View::Detail {
        detail: DetailView {
            title: None,
            content: DetailContent::Text {
                value: "content".to_owned(),
            },
            metadata: Vec::new(),
            actions: Vec::new(),
        },
    };

    authorize_view_event(&view, &ViewEvent::Resumed)
        .expect("an active extension view should receive resume lifecycle events");
}
