use crate::ClipboardViewState;
use nanika_protocol::{DetailContent, DetailView, View, ViewItemIcon};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct ClipboardPresentation {
    pub state: ClipboardViewState,
    pub view: View,
    pub data_revision: u64,
    /// Clear acts on the reviewed query scope, including rows outside the current page.
    pub matching_ids: Arc<Vec<String>>,
    pub(crate) paths: HashMap<String, Vec<String>>,
}

impl ClipboardPresentation {
    pub fn decorate_icons(
        &mut self,
        icon_for_path: &impl Fn(&Path) -> Option<Option<nanika_protocol::IconReference>>,
    ) {
        let View::List { list } = &mut self.view else {
            unreachable!()
        };
        let selected = list.selected_item_id.as_ref();
        let selected_references = selected
            .and_then(|id| self.paths.get(id))
            .into_iter()
            .flatten()
            .map(|path| icon_for_path(Path::new(path)))
            .collect::<Vec<_>>();
        for item in list
            .sections
            .iter_mut()
            .flat_map(|section| &mut section.items)
        {
            let reference = if selected == Some(&item.id) {
                selected_references.first().cloned().flatten().flatten()
            } else {
                self.paths
                    .get(&item.id)
                    .and_then(|paths| paths.first())
                    .and_then(|path| icon_for_path(Path::new(path)))
                    .flatten()
            };
            if let Some(reference) = reference {
                item.icon = Some(ViewItemIcon::Native(reference));
            }
        }
        if let Some(DetailView {
            content: DetailContent::Files { files },
            ..
        }) = &mut list.detail
            && selected_references.iter().all(Option::is_some)
        {
            for (file, reference) in files.iter_mut().zip(selected_references) {
                file.icon = reference.flatten();
            }
        }
    }

    pub fn icon_paths(&self) -> Vec<PathBuf> {
        let selected = self.state.selected_item_id.as_ref();
        let first = selected
            .and_then(|id| self.paths.get(id))
            .into_iter()
            .flatten();
        let View::List { list } = &self.view else {
            unreachable!()
        };
        let others = list
            .sections
            .iter()
            .flat_map(|section| &section.items)
            .filter(|item| Some(&item.id) != selected)
            .filter_map(|item| self.paths.get(&item.id))
            .flatten();
        first.chain(others).map(PathBuf::from).collect()
    }
}
