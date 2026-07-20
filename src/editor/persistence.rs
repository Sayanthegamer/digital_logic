use crate::engine::{ChipBlueprint, ComponentType};
use crate::editor::color_coding::ColorOverrides;
use crate::editor::global_library;


use super::Editor;
use super::types::*;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProjectFile {
    pub library: Vec<ChipBlueprint>,
    pub components: Vec<VisualComponent>,
    pub connections: Vec<VisualConnection>,
    pub next_component_id: usize,
    pub annotations: Vec<TextAnnotation>,
    /// Per-component and per-wire colour overrides (backward compat: defaults to empty)
    #[serde(default)]
    pub color_overrides: ColorOverrides,
    /// Manual routing nudges per wire
    #[serde(default)]
    pub wire_nudges: Vec<(crate::editor::types::VisualConnection, f32)>,
}

#[derive(serde::Serialize)]
pub struct ProjectFileRef<'a> {
    pub library: &'a [ChipBlueprint],
    pub components: &'a [VisualComponent],
    pub connections: &'a [VisualConnection],
    pub next_component_id: usize,
    pub annotations: &'a [TextAnnotation],
    pub color_overrides: &'a ColorOverrides,
    pub wire_nudges: Vec<(&'a crate::editor::types::VisualConnection, &'a f32)>,
}

impl Editor {
    pub(crate) fn save_to_path<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let project = ProjectFileRef {
            library: &self.engine.library,
            components: &self.circuit.components,
            connections: &self.circuit.connections,
            next_component_id: self.circuit.next_component_id,
            annotations: &self.circuit.annotations,
            color_overrides: &self.circuit.color_overrides,
            wire_nudges: self.circuit.wire_nudges.iter().collect(),
        };

        let serialized = serde_json::to_string_pretty(&project)?;
        let mut file = std::fs::File::create(path)?;
        use std::io::Write;
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }

    pub(crate) fn load_from_path<P: AsRef<std::path::Path>>(&mut self, path: P) -> bool {
        // Cap file reads at 50 MB to prevent OOM from maliciously large files.
        const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
        if let Ok(file) = std::fs::File::open(path) {
            let mut contents = String::new();
            use std::io::Read;
            if file
                .take(MAX_FILE_SIZE)
                .read_to_string(&mut contents)
                .is_ok()
                && let Ok(mut project) = serde_json::from_str::<ProjectFile>(&contents)
            {
                // MIGRATION STEP: clock_period to bus_width for BusJoiner/BusSplitter
                for comp in &mut project.components {
                    if (comp.comp_type == ComponentType::BusJoiner || comp.comp_type == ComponentType::BusSplitter) && comp.bus_width.is_none() {
                        comp.bus_width = comp.clock_period;
                    }
                }
                for bp in &mut project.library {
                    for comp in &mut bp.components {
                        if (comp.component_type == ComponentType::BusJoiner || comp.component_type == ComponentType::BusSplitter) && comp.bus_width.is_none() {
                            comp.bus_width = comp.clock_period;
                        }
                    }
                }

                // Import project-local chips into the global library and get index mapping
                let index_map = self.global_library.import_from_project(&project.library);
                global_library::save_global_library(&self.global_library);

                // Rebuild engine library from global library
                self.engine.library = self.global_library.to_flat_list();

                self.circuit.components = project.components;

                // Remap subchip indices of loaded components to match the new flat library indices
                for comp in &mut self.circuit.components {
                    if let ComponentType::SubChip(ref mut sub_idx) = comp.comp_type {
                        if let Some(&new_sub_idx) = index_map.get(sub_idx) {
                            *sub_idx = new_sub_idx;
                        }
                    }
                }

                self.circuit.connections = project.connections;
                self.circuit.next_component_id = project.next_component_id;
                self.circuit.annotations = project.annotations;
                self.circuit.color_overrides = project.color_overrides;
                self.circuit.wire_nudges = project.wire_nudges.into_iter().collect();
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
                if let Err(err) = self.save_to_path(path) {
                    eprintln!("Failed to save project: {err}");
                }
            }
        }

        #[cfg(target_os = "android")]
        {
            let path = match get_android_external_files_dir()
                .or_else(|ext_err| {
                    eprintln!("Failed to resolve external files dir: {ext_err}");
                    get_android_internal_files_dir()
                }) {
                Ok(dir) => dir,
                Err(err) => {
                    eprintln!("Failed to resolve Android files dir: {err}");
                    return;
                }
            };

            let mut save_path = path;
            save_path.push("project_save.logic");
            if let Err(err) = self.save_to_path(save_path) {
                eprintln!("Failed to save project: {err}");
            }
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
            let external_dir = match get_android_external_files_dir() {
                Ok(dir) => Some(dir),
                Err(err) => {
                    eprintln!("Failed to resolve external files dir: {err}");
                    None
                }
            };
            let internal_dir = match get_android_internal_files_dir() {
                Ok(dir) => Some(dir),
                Err(err) => {
                    eprintln!("Failed to resolve internal files dir: {err}");
                    None
                }
            };

            let mut external_path = external_dir.clone();
            if let Some(ref mut p) = external_path {
                p.push("project_save.logic");
            }
            let mut internal_path = internal_dir.clone();
            if let Some(ref mut p) = internal_path {
                p.push("project_save.logic");
            }

            // Prefer loading from the new external directory.
            if let Some(ref p) = external_path
                && p.exists()
                && self.load_from_path(p)
            {
                return true;
            }

            // Fallback to the legacy internal directory.
            if let Some(ref p) = internal_path
                && p.exists()
                && self.load_from_path(p)
            {
                // Best-effort migration to the new external location.
                if let (Some(external_dir), Some(external_path)) = (external_dir, external_path) {
                    if let Err(err) = std::fs::create_dir_all(&external_dir) {
                        eprintln!("Failed to create external files dir {external_dir:?}: {err}");
                    } else if let Err(err) = std::fs::copy(p, &external_path) {
                        eprintln!("Failed to migrate save to {external_path:?}: {err}");
                    } else if let Err(err) = std::fs::remove_file(p) {
                        eprintln!("Failed to remove legacy save {p:?}: {err}");
                    }
                }
                return true;
            }

            false
        }
    }
}

#[cfg(target_os = "android")]
pub(crate) fn get_android_external_files_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    use jni::objects::{JObject, JValue};

    let env_ptr = unsafe { miniquad::native::android::attach_jni_env() };
    if env_ptr.is_null() {
        return Err("attach_jni_env returned null".into());
    }
    let mut env = unsafe { jni::JNIEnv::from_raw(env_ptr as *mut jni::sys::JNIEnv)? };

    let activity_ptr = unsafe { miniquad::native::android::ACTIVITY };
    if activity_ptr.is_null() {
        return Err("ACTIVITY jobject is null".into());
    }
    let context_obj = unsafe { JObject::from_raw(activity_ptr as jni::sys::jobject) };

    let file_obj = env
        .call_method(
            &context_obj,
            "getExternalFilesDir",
            "(Ljava/lang/String;)Ljava/io/File;",
            &[JValue::Object(JObject::null().as_ref())],
        )?
        .l()?;
    if file_obj.is_null() {
        return Err("getExternalFilesDir(null) returned null".into());
    }

    let path_jstring = env
        .call_method(&file_obj, "getAbsolutePath", "()Ljava/lang/String;", &[])?
        .l()?;
    let path: String = env.get_string((&path_jstring).into())?.into();
    Ok(std::path::PathBuf::from(path))
}

#[cfg(target_os = "android")]
pub(crate) fn get_android_internal_files_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    use jni::objects::JObject;

    let env_ptr = unsafe { miniquad::native::android::attach_jni_env() };
    if env_ptr.is_null() {
        return Err("attach_jni_env returned null".into());
    }
    let mut env = unsafe { jni::JNIEnv::from_raw(env_ptr as *mut jni::sys::JNIEnv)? };

    let activity_ptr = unsafe { miniquad::native::android::ACTIVITY };
    if activity_ptr.is_null() {
        return Err("ACTIVITY jobject is null".into());
    }
    let context_obj = unsafe { JObject::from_raw(activity_ptr as jni::sys::jobject) };

    let file_obj = env
        .call_method(&context_obj, "getFilesDir", "()Ljava/io/File;", &[])?
        .l()?;
    if file_obj.is_null() {
        return Err("getFilesDir() returned null".into());
    }

    let path_jstring = env
        .call_method(&file_obj, "getAbsolutePath", "()Ljava/lang/String;", &[])?
        .l()?;
    let path: String = env.get_string((&path_jstring).into())?.into();
    Ok(std::path::PathBuf::from(path))
}
