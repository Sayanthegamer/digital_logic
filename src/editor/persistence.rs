use crate::engine::ChipBlueprint;

use super::Editor;
use super::types::*;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProjectFile {
    pub library: Vec<ChipBlueprint>,
    pub components: Vec<VisualComponent>,
    pub connections: Vec<VisualConnection>,
    pub next_component_id: usize,
    pub annotations: Vec<TextAnnotation>,
}

impl Editor {
    pub fn save_project(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Logic Simulator Projects", &["json"])
            .set_directory(".")
            .save_file()
        {
            let project = ProjectFile {
                library: self.library.clone(),
                components: self.components.clone(),
                connections: self.connections.clone(),
                next_component_id: self.next_component_id,
                annotations: self.annotations.clone(),
            };
            if let Ok(serialized) = serde_json::to_string_pretty(&project) {
                if let Ok(mut file) = std::fs::File::create(path) {
                    use std::io::Write;
                    let _ = file.write_all(serialized.as_bytes());
                }
            }
        }
    }

    pub fn load_project(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Logic Simulator Projects", &["json"])
            .set_directory(".")
            .pick_file() {
            if let Ok(mut file) = std::fs::File::open(path) {
                let mut contents = String::new();
                use std::io::Read;
                if file.read_to_string(&mut contents).is_ok() {
                    if let Ok(project) = serde_json::from_str::<ProjectFile>(&contents) {
                        self.library = project.library;
                        self.components = project.components;
                        self.connections = project.connections;
                        self.next_component_id = project.next_component_id;
                        self.annotations = project.annotations;
                        self.selected_comp_id = None;
                        self.selected_annotation_idx = None;
                        self.inspection_path.clear();
                        self.compile();
                    }
                }
            }
        }
    }
}
