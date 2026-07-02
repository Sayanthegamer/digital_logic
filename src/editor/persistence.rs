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
    pub(crate) fn save_to_path<P: AsRef<std::path::Path>>(&self, path: P) {
        let project = ProjectFile {
            library: self.engine.library.clone(),
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

    pub(crate) fn load_from_path<P: AsRef<std::path::Path>>(&mut self, path: P) -> bool {
        // Cap file reads at 50 MB to prevent OOM from maliciously large files.
        const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
        if let Ok(file) = std::fs::File::open(path) {
            let mut contents = String::new();
            use std::io::Read;
            if file.take(MAX_FILE_SIZE).read_to_string(&mut contents).is_ok()
                && let Ok(project) = serde_json::from_str::<ProjectFile>(&contents)
            {
                self.engine.library = project.library;
                self.components = project.components;
                self.connections = project.connections;
                self.next_component_id = project.next_component_id;
                self.annotations = project.annotations;
                self.canvas.selected_comp_id = None;
                self.canvas.selected_annotation_idx = None;
                self.canvas.inspection_path.clear();
                self.compile();
                return true;
            }
        }
        false
    }

    pub fn save_project(&self) {
        #[cfg(not(target_os = "android"))]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Logic Simulator Projects", &["logic", "json"])
                .set_directory(".")
                .save_file()
            {
                self.save_to_path(path);
            }
        }

        #[cfg(target_os = "android")]
        {
            let mut path = get_android_files_dir();
            path.push("project_save.logic");
            self.save_to_path(path);
        }
    }

    pub fn load_project(&mut self) -> bool {
        #[cfg(not(target_os = "android"))]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Logic Simulator Projects", &["logic", "json"])
                .set_directory(".")
                .pick_file()
            {
                return self.load_from_path(path);
            }
            false
        }

        #[cfg(target_os = "android")]
        {
            let mut path = get_android_files_dir();
            path.push("project_save.logic");
            self.load_from_path(path)
        }
    }
}

#[cfg(target_os = "android")]
fn get_android_files_dir() -> std::path::PathBuf {
    use jni::objects::JObject;

    let get_path = || -> Result<String, Box<dyn std::error::Error>> {
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
        let mut env = vm.attach_current_thread()?;
        let context_obj = unsafe { JObject::from_raw(ctx.context() as jni::sys::jobject) };
        let file_obj = env.call_method(context_obj, "getFilesDir", "()Ljava/io/File;", &[])?.l()?;
        let path_jstring = env.call_method(file_obj, "getAbsolutePath", "()Ljava/lang/String;", &[])?.l()?;
        let path: String = env.get_string(path_jstring.into())?.into();
        Ok(path)
    };

    get_path()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}
