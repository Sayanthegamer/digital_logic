use crate::engine::{ChipBlueprint, ComponentType};
use crate::editor::color_coding::ColorOverrides;
use crate::editor::global_library;
use crate::editor::theme;

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
}

impl Editor {
    pub(crate) fn save_to_path<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let project = ProjectFile {
            library: self.engine.library.clone(),
            components: self.components.clone(),
            connections: self.connections.clone(),
            next_component_id: self.next_component_id,
            annotations: self.annotations.clone(),
            color_overrides: self.color_overrides.clone(),
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
                && let Ok(project) = serde_json::from_str::<ProjectFile>(&contents)
            {
                // Import project-local chips into the global library and get index mapping
                let index_map = self.global_library.import_from_project(&project.library);
                global_library::save_global_library(&self.global_library);

                // Rebuild engine library from global library
                self.engine.library = self.global_library.to_flat_list();

                self.components = project.components;

                // Remap subchip indices of loaded components to match the new flat library indices
                for comp in &mut self.components {
                    if let ComponentType::SubChip(ref mut sub_idx) = comp.comp_type {
                        if let Some(&new_sub_idx) = index_map.get(sub_idx) {
                            *sub_idx = new_sub_idx;
                        }
                    }
                }

                self.connections = project.connections;
                self.next_component_id = project.next_component_id;
                self.annotations = project.annotations;
                self.color_overrides = project.color_overrides;
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

    pub(crate) fn export_svg_to_path<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;
        
        if self.components.is_empty() {
            return Err("No components to export".into());
        }

        // 1. Find bounding box of all components and annotations
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for comp in &self.components {
            min_x = min_x.min(comp.pos.x);
            min_y = min_y.min(comp.pos.y);
            max_x = max_x.max(comp.pos.x + comp.width);
            max_y = max_y.max(comp.pos.y + comp.height);
        }

        for ann in &self.annotations {
            min_x = min_x.min(ann.pos.x);
            min_y = min_y.min(ann.pos.y);
            max_x = max_x.max(ann.pos.x + 150.0);
            max_y = max_y.max(ann.pos.y + 20.0);
        }

        let padding = 40.0;
        min_x -= padding;
        min_y -= padding;
        max_x += padding;
        max_y += padding;

        let svg_width = max_x - min_x;
        let svg_height = max_y - min_y;

        // Helper to format ThemeColor to Hex
        let to_hex = |color: &theme::ThemeColor| -> String {
            format!(
                "#{:02X}{:02X}{:02X}",
                (color.r * 255.0).round() as u8,
                (color.g * 255.0).round() as u8,
                (color.b * 255.0).round() as u8
            )
        };

        let mut svg = String::new();
        svg.push_str(&format!(
            r#"<svg width="{}" height="{}" viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">
"#,
            svg_width, svg_height, svg_width, svg_height
        ));

        // Add CSS style block
        svg.push_str(&format!(
            r#"  <style>
    rect.bg {{ fill: {}; }}
    rect.component {{ fill: {}; stroke: {}; stroke-width: 1.5px; rx: 6px; }}
    circle.port {{ fill: {}; }}
    path.wire {{ fill: none; stroke-linecap: round; stroke-linejoin: round; }}
    text {{ font-family: 'Inter', 'Segoe UI', sans-serif; font-size: 13px; fill: {}; font-weight: 500; }}
    text.port-label {{ font-size: 10px; fill: {}; }}
    text.annotation {{ font-size: 15px; fill: {}; }}
  </style>
  <rect class="bg" width="100%" height="100%"/>
"#,
            to_hex(&theme::BG_CANVAS),
            to_hex(&theme::BG_PANEL),
            to_hex(&theme::BORDER),
            to_hex(&theme::ACCENT_GENERIC),
            to_hex(&theme::TEXT_PRIMARY),
            to_hex(&theme::TEXT_SECONDARY),
            to_hex(&theme::TEXT_SECONDARY)
        ));

        // 2. Draw connections (so they render behind components)
        for wire in &self.connections {
            let src_comp = self.components.iter().find(|c| c.id == wire.src_comp_id);
            let tgt_comp = self.components.iter().find(|c| c.id == wire.tgt_comp_id);

            if let (Some(src), Some(tgt)) = (src_comp, tgt_comp) {
                let (_, src_outputs) = self.get_component_ports_count_with_width(src.comp_type, Some(src.bus_width()));
                let (tgt_inputs, _) = self.get_component_ports_count_with_width(tgt.comp_type, Some(tgt.bus_width()));

                let src_pos = src.output_port_pos(wire.src_port, src_outputs);
                let tgt_pos = tgt.input_port_pos(wire.tgt_port, tgt_inputs);

                let offset = self.get_connection_routing_offset(wire);
                let segments = Self::compute_wire_segments_world(src_pos, tgt_pos, offset);
                let is_bus = self.is_bus_connection(wire);

                let wire_state = if let Some(&gate_idx) = self
                    .engine
                    .port_to_sim_gate_map
                    .get(&(wire.src_comp_id, wire.src_port))
                {
                    self.engine.simulator.get_raw_state(gate_idx)
                } else if src.comp_type == ComponentType::Input {
                    if let Some(&gate_idx) = self.engine.visual_to_sim_map.get(&src.id) {
                        self.engine.simulator.get_raw_state(gate_idx)
                    } else {
                        0b00
                    }
                } else {
                    0b00
                };

                let (wire_color, thickness) = match wire_state {
                    0b00 => (theme::ACCENT_GENERIC, 1.3),
                    0b01 => (theme::ACCENT_INACTIVE, 1.6),
                    0b10 => (theme::ACCENT_PRIMARY, 2.2),
                    _ => (theme::COMP_NAND, 2.8),
                };

                let wire_color_hex = to_hex(&wire_color);
                let final_thickness = if is_bus { thickness * 2.2 } else { thickness };

                let mut d_attr = String::new();
                for (i, &(a, b)) in segments.iter().enumerate() {
                    let ax = a.x - min_x;
                    let ay = a.y - min_y;
                    let bx = b.x - min_x;
                    let by = b.y - min_y;
                    if i == 0 {
                        d_attr.push_str(&format!("M {} {} L {} {}", ax, ay, bx, by));
                    } else {
                        d_attr.push_str(&format!(" L {} {}", bx, by));
                    }
                }

                svg.push_str(&format!(
                    r#"  <path class="wire" d="{}" stroke="{}" stroke-width="{}" />
"#,
                    d_attr, wire_color_hex, final_thickness
                ));
            }
        }

        // 3. Draw components
        for comp in &self.components {
            let cx = comp.pos.x - min_x;
            let cy = comp.pos.y - min_y;

            // Box shadow
            svg.push_str(&format!(
                r#"  <rect x="{}" y="{}" width="{}" height="{}" fill="rgba(0,0,0,0.25)" rx="6" />
"#,
                cx + 3.0, cy + 3.0, comp.width, comp.height
            ));

            // Component body
            svg.push_str(&format!(
                r#"  <rect class="component" x="{}" y="{}" width="{}" height="{}" />
"#,
                cx, cy, comp.width, comp.height
            ));

            // Accent stripe
            let accent_color = if let Some(c) = self.color_overrides.get_component_color(comp.id) {
                to_hex(&theme::ThemeColor::new(c.r, c.g, c.b, c.a))
            } else {
                let theme_color = match comp.comp_type {
                    ComponentType::Nand => theme::COMP_NAND,
                    ComponentType::Clock => theme::ACCENT_PRIMARY,
                    ComponentType::Input | ComponentType::Output => {
                        let is_active = if let Some(&gate_idx) = self.engine.visual_to_sim_map.get(&comp.id) {
                            self.engine.simulator.get_state(gate_idx)
                        } else {
                            false
                        };
                        if is_active { theme::ACCENT_ACTIVE } else { theme::ACCENT_GENERIC }
                    }
                    ComponentType::SubChip(_) => theme::COMP_SUBCHIP,
                    ComponentType::SevenSegment => theme::COMP_SEVENSEG,
                    ComponentType::TriStateBuffer => theme::COMP_NAND,
                    _ => theme::ACCENT_GENERIC,
                };
                to_hex(&theme_color)
            };

            let stripe_height = 4.0;
            // Draw top accent bar as a path with top-left/top-right rounded corners matching border radius
            svg.push_str(&format!(
                r#"  <path d="M {cx} {cy_stripe} a 6 6 0 0 1 6 -6 h {w_accent} a 6 6 0 0 1 6 6 v 2 h -{w_full} z" fill="{color}" />
"#,
                cx = cx,
                cy_stripe = cy + stripe_height,
                w_accent = comp.width - 12.0,
                w_full = comp.width,
                color = accent_color
            ));

            // Component label
            if comp.comp_type != ComponentType::SevenSegment {
                let lx = cx + comp.width / 2.0;
                let ly = cy + comp.height / 2.0 + 4.0;
                svg.push_str(&format!(
                    r#"  <text x="{}" y="{}" text-anchor="middle">{}</text>
"#,
                    lx, ly, comp.label
                ));
            }

            // Port circles
            let (inputs_count, outputs_count) = self.get_component_ports_count_with_width(comp.comp_type, Some(comp.bus_width()));
            for i in 0..inputs_count {
                let p = comp.input_port_pos(i, inputs_count);
                svg.push_str(&format!(
                    r#"  <circle class="port" cx="{}" cy="{}" r="4" />
"#,
                    p.x - min_x, p.y - min_y
                ));
            }
            for o in 0..outputs_count {
                let p = comp.output_port_pos(o, outputs_count);
                svg.push_str(&format!(
                    r#"  <circle class="port" cx="{}" cy="{}" r="4" />
"#,
                    p.x - min_x, p.y - min_y
                ));
            }

            // Custom SubChip Port Names
            if let ComponentType::SubChip(idx) = comp.comp_type
                && let Some(bp) = self.engine.library.get(idx)
            {
                for i in 0..inputs_count {
                    let p = comp.input_port_pos(i, inputs_count);
                    let name = bp.input_names.get(i).cloned().unwrap_or_else(|| format!("{}", i));
                    svg.push_str(&format!(
                        r#"  <text class="port-label" x="{}" y="{}">{}</text>
"#,
                        p.x - min_x + 6.0, p.y - min_y + 3.0, name
                    ));
                }
                for o in 0..outputs_count {
                    let p = comp.output_port_pos(o, outputs_count);
                    let name = bp.output_names.get(o).cloned().unwrap_or_else(|| format!("{}", o));
                    svg.push_str(&format!(
                        r#"  <text class="port-label" x="{}" y="{}" text-anchor="end">{}</text>
"#,
                        p.x - min_x - 6.0, p.y - min_y + 3.0, name
                    ));
                }
            }

            // SevenSegment display segments
            if comp.comp_type == ComponentType::SevenSegment {
                let mut seg_states = [false; 8];
                for i in 0..inputs_count {
                    let mut input_active = false;
                    for wire in &self.connections {
                        if wire.tgt_comp_id == comp.id && wire.tgt_port == i {
                            if let Some(&gate_idx) = self.engine.port_to_sim_gate_map.get(&(wire.src_comp_id, wire.src_port)) {
                                input_active = self.engine.simulator.get_state(gate_idx);
                            } else if let Some(src_comp) = self.components.iter().find(|c| c.id == wire.src_comp_id) {
                                if src_comp.comp_type == ComponentType::Input {
                                    if let Some(&gate_idx) = self.engine.visual_to_sim_map.get(&src_comp.id) {
                                        input_active = self.engine.simulator.get_state(gate_idx);
                                    }
                                }
                            }
                        }
                    }
                    if i < 8 {
                        seg_states[i] = input_active;
                    }
                }

                let cx_seg = cx + comp.width / 2.0;
                let cy_seg = cy + comp.height / 2.0;
                let w = 15.0;
                let h = 15.0;
                let thick = 4.0;
                let seg_color = |active| if active { to_hex(&theme::COMP_SEVENSEG) } else { format!("{}1A", to_hex(&theme::COMP_SEVENSEG)) };

                let draw_seg = |svg_str: &mut String, x1, y1, x2, y2, active| {
                    svg_str.push_str(&format!(
                        r#"  <line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" stroke-linecap="round" />
"#,
                        x1, y1, x2, y2, seg_color(active), thick
                    ));
                };

                draw_seg(&mut svg, cx_seg - w, cy_seg - 2.0 * h, cx_seg + w, cy_seg - 2.0 * h, seg_states[0]);
                draw_seg(&mut svg, cx_seg + w, cy_seg - 2.0 * h, cx_seg + w, cy_seg, seg_states[1]);
                draw_seg(&mut svg, cx_seg + w, cy_seg, cx_seg + w, cy_seg + 2.0 * h, seg_states[2]);
                draw_seg(&mut svg, cx_seg - w, cy_seg + 2.0 * h, cx_seg + w, cy_seg + 2.0 * h, seg_states[3]);
                draw_seg(&mut svg, cx_seg - w, cy_seg, cx_seg - w, cy_seg + 2.0 * h, seg_states[4]);
                draw_seg(&mut svg, cx_seg - w, cy_seg - 2.0 * h, cx_seg - w, cy_seg, seg_states[5]);
                draw_seg(&mut svg, cx_seg - w, cy_seg, cx_seg + w, cy_seg, seg_states[6]);
                draw_seg(&mut svg, cx_seg - w - 20.0, cy_seg, cx_seg - w - 10.0, cy_seg, seg_states[7]);
            }
        }

        // 4. Draw Annotations
        for ann in &self.annotations {
            let ax = ann.pos.x - min_x;
            let ay = ann.pos.y - min_y;
            svg.push_str(&format!(
                r#"  <text class="annotation" x="{}" y="{}">{}</text>
"#,
                ax, ay, ann.text
            ));
        }

        svg.push_str("</svg>\n");

        let mut file = std::fs::File::create(path)?;
        file.write_all(svg.as_bytes())?;
        Ok(())
    }

    pub fn export_to_svg(&self) {
        #[cfg(not(target_os = "android"))]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("SVG Vector Graphics", &["svg"])
                .set_directory(".")
                .save_file()
            {
                if let Err(err) = self.export_svg_to_path(path) {
                    eprintln!("Failed to export SVG: {err}");
                }
            }
        }

        #[cfg(target_os = "android")]
        {
            if let Ok(dir) = get_android_external_files_dir().or_else(|_| get_android_internal_files_dir()) {
                let mut path = dir;
                path.push("project_export.svg");
                if let Err(err) = self.export_svg_to_path(path) {
                    eprintln!("Failed to export SVG: {err}");
                }
            }
        }
    }
}

#[cfg(target_os = "android")]
fn get_android_external_files_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    use jni::objects::{JObject, JValue};

    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
    let mut env = vm.attach_current_thread()?;
    let context_obj = unsafe { JObject::from_raw(ctx.context() as jni::sys::jobject) };

    let file_obj = env
        .call_method(
            context_obj,
            "getExternalFilesDir",
            "(Ljava/lang/String;)Ljava/io/File;",
            &[JValue::Object(JObject::null().as_ref())],
        )?
        .l()?;
    if file_obj.is_null() {
        return Err("getExternalFilesDir(null) returned null".into());
    }

    let path_jstring = env
        .call_method(file_obj, "getAbsolutePath", "()Ljava/lang/String;", &[])?
        .l()?;
    let path: String = env.get_string((&path_jstring).into())?.into();
    Ok(std::path::PathBuf::from(path))
}

#[cfg(target_os = "android")]
fn get_android_internal_files_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    use jni::objects::JObject;

    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
    let mut env = vm.attach_current_thread()?;
    let context_obj = unsafe { JObject::from_raw(ctx.context() as jni::sys::jobject) };

    let file_obj = env
        .call_method(context_obj, "getFilesDir", "()Ljava/io/File;", &[])?
        .l()?;
    if file_obj.is_null() {
        return Err("getFilesDir() returned null".into());
    }

    let path_jstring = env
        .call_method(file_obj, "getAbsolutePath", "()Ljava/lang/String;", &[])?
        .l()?;
    let path: String = env.get_string((&path_jstring).into())?.into();
    Ok(std::path::PathBuf::from(path))
}
