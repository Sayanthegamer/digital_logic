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
    fn save_to_path<P: AsRef<std::path::Path>>(&self, path: P) {
        let project = ProjectFile {
            library: self.library.clone(),
            components: self.components.clone(),
            connections: self.connections.clone(),
            next_component_id: self.next_component_id,
            annotations: self.annotations.clone(),
        };
        if let Ok(serialized) = serde_json::to_string_pretty(&project)
            && let Ok(mut file) = std::fs::File::create(path)
        {
            use std::io::Write;
            let _ = file.write_all(serialized.as_bytes());
        }
    }

    fn load_from_path<P: AsRef<std::path::Path>>(&mut self, path: P) {
        if let Ok(mut file) = std::fs::File::open(path) {
            let mut contents = String::new();
            use std::io::Read;
            if file.read_to_string(&mut contents).is_ok()
                && let Ok(project) = serde_json::from_str::<ProjectFile>(&contents)
            {
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

    pub fn save_project(&self) {
        #[cfg(not(target_os = "android"))]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Logic Simulator Projects", &["json"])
                .set_directory(".")
                .save_file()
            {
                self.save_to_path(path);
            }
        }

        #[cfg(target_os = "android")]
        {
            // On Android, save to a fixed local file since RFD is not supported
            let path = std::path::PathBuf::from("project_save.json");
            self.save_to_path(path);
        }
    }

    pub fn load_project(&mut self) {
        #[cfg(not(target_os = "android"))]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Logic Simulator Projects", &["json"])
                .set_directory(".")
                .pick_file()
            {
                self.load_from_path(path);
            }
        }

        #[cfg(target_os = "android")]
        {
            // On Android, load from the fixed local file since RFD is not supported
            let path = std::path::PathBuf::from("project_save.json");
            self.load_from_path(path);
        }
    }
}
