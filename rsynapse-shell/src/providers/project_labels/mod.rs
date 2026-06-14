mod format;
mod hydrate;
mod provider;

use locus_provider::NodeId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectLabelView {
    pub project_id: Option<NodeId>,
    pub sort_index: u32,
    pub icon: String,
    pub primary: String,
    pub secondary: String,
    pub tooltip: String,
    pub urgent: bool,
}

impl Default for ProjectLabelView {
    fn default() -> Self {
        Self {
            project_id: None,
            sort_index: 0,
            icon: "workspaces".to_owned(),
            primary: String::new(),
            secondary: String::new(),
            tooltip: String::new(),
            urgent: false,
        }
    }
}

pub(crate) use provider::project_label_for_workspace;
