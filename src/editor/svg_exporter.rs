use crate::engine::ComponentType;
use crate::editor::theme;
use super::Editor;

#[cfg(target_os = "android")]
use super::persistence::{get_android_external_files_dir, get_android_internal_files_dir};

fn draw_seg(svg_str: &mut String, x1: f32, y1: f32, x2: f32, y2: f32, active: bool, thick: f32) {
    let seg_color = if active { 
        theme::COMP_SEVENSEG.to_hex() 
    } else { 
        format!("{}1A", theme::COMP_SEVENSEG.to_hex()) 
    };
    svg_str.push_str(&format!(
        r#"  <line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" stroke-linecap="round" />
"#,
        x1, y1, x2, y2, seg_color, thick
    ));
}

impl Editor {
    pub(crate) fn export_svg_to_path<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;
        
        if self.circuit.components.is_empty() {
            return Err("No components to export".into());
        }

        // 1. Find bounding box of all components and annotations
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for comp in &self.circuit.components {
            min_x = min_x.min(comp.pos.x);
            min_y = min_y.min(comp.pos.y);
            max_x = max_x.max(comp.pos.x + comp.width);
            max_y = max_y.max(comp.pos.y + comp.height);
        }

        for ann in &self.circuit.annotations {
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
            theme::BG_CANVAS.to_hex(),
            theme::BG_PANEL.to_hex(),
            theme::BORDER.to_hex(),
            theme::ACCENT_GENERIC.to_hex(),
            theme::TEXT_PRIMARY.to_hex(),
            theme::TEXT_SECONDARY.to_hex(),
            theme::TEXT_SECONDARY.to_hex()
        ));

        // 2. Draw connections (so they render behind components)
        for wire in &self.circuit.connections {
            let src_comp = self.get_component(wire.src_comp_id);
            let tgt_comp = self.get_component(wire.tgt_comp_id);

            if let (Some(src), Some(tgt)) = (src_comp, tgt_comp) {
                let (_, src_outputs) = self.get_component_ports_count_with_width(src.comp_type, Some(src.bus_width()));
                let (tgt_inputs, _) = self.get_component_ports_count_with_width(tgt.comp_type, Some(tgt.bus_width()));

                let src_pos = src.output_port_pos(wire.src_port, src_outputs);
                let tgt_pos = tgt.input_port_pos(wire.tgt_port, tgt_inputs);

                let offset = self.get_connection_routing_offset(wire);
                let segments = Self::compute_wire_segments_world(src_pos, tgt_pos, offset, wire.tgt_port);
                let is_bus = self.is_bus_connection(wire);

                let wire_state = self.get_raw_wire_state(wire.src_comp_id, wire.src_port);

                let (wire_color, thickness) = match wire_state {
                    0b00 => (theme::ACCENT_GENERIC, 1.3),
                    0b01 => (theme::ACCENT_INACTIVE, 1.6),
                    0b10 => (theme::ACCENT_PRIMARY, 2.2),
                    _ => (theme::COMP_NAND, 2.8),
                };

                let wire_color_hex = wire_color.to_hex();
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
        for comp in &self.circuit.components {
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
            let accent_color = if let Some(c) = self.circuit.color_overrides.get_component_color(comp.id) {
                theme::ThemeColor::new(c.r, c.g, c.b, c.a).to_hex()
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
                theme_color.to_hex()
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
                    for wire in &self.circuit.connections {
                        if wire.tgt_comp_id == comp.id && wire.tgt_port == i {
                            input_active = self.get_wire_state(wire.src_comp_id, wire.src_port);
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

                draw_seg(&mut svg, cx_seg - w, cy_seg - 2.0 * h, cx_seg + w, cy_seg - 2.0 * h, seg_states[0], thick);
                draw_seg(&mut svg, cx_seg + w, cy_seg - 2.0 * h, cx_seg + w, cy_seg, seg_states[1], thick);
                draw_seg(&mut svg, cx_seg + w, cy_seg, cx_seg + w, cy_seg + 2.0 * h, seg_states[2], thick);
                draw_seg(&mut svg, cx_seg - w, cy_seg + 2.0 * h, cx_seg + w, cy_seg + 2.0 * h, seg_states[3], thick);
                draw_seg(&mut svg, cx_seg - w, cy_seg, cx_seg - w, cy_seg + 2.0 * h, seg_states[4], thick);
                draw_seg(&mut svg, cx_seg - w, cy_seg - 2.0 * h, cx_seg - w, cy_seg, seg_states[5], thick);
                draw_seg(&mut svg, cx_seg - w, cy_seg, cx_seg + w, cy_seg, seg_states[6], thick);
                draw_seg(&mut svg, cx_seg - w - 20.0, cy_seg, cx_seg - w - 10.0, cy_seg, seg_states[7], thick);
            }
        }

        // 4. Draw Annotations
        for ann in &self.circuit.annotations {
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
