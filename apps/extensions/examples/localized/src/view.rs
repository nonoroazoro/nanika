use nanika_protocol::{DetailContent, DetailView, View, ViewFile, ViewMetadata, message};

pub(crate) fn files() -> View {
    // Sample domain content stays literal. These display paths do not authorize file access.
    let names = ["report.pdf", "budget.csv", "notes.txt"];
    View::Detail {
        detail: DetailView {
            title: Some(message(
                "files-more",
                [
                    ("name".into(), names[0].into()),
                    ("count".into(), ((names.len() - 1) as i64).into())
                ]
            )),
            content: DetailContent::Files {
                files: names
                    .into_iter()
                    .map(|name| ViewFile {
                        name: name.into(),
                        path: format!("/Example/{name}"),
                        icon: None
                    })
                    .collect()
            },
            metadata: vec![ViewMetadata {
                title: message("metadata-count", []),
                value: message(
                    "item-count",
                    [("count".into(), (names.len() as i64).into())]
                )
                .into()
            }],
            actions: Vec::new()
        }
    }
}
