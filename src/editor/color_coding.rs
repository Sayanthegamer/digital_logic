use macroquad::color::Color;
use std::collections::HashMap;

use super::types::VisualConnection;

/// What the right-click context menu is targeting.
#[derive(Clone, Debug)]
pub enum ContextMenuTarget {
    Component(usize),          // visual component ID
    Wire(VisualConnection),    // the connection
}

/// Stores per-component and per-wire colour overrides.
/// These are layered on top of the theme defaults.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ColorOverrides {
    /// Visual component ID → RGBA colour override (for the accent stripe / body)
    #[serde(default)]
    pub component_colors: HashMap<usize, [f32; 4]>,
    /// Connection (serialised as "(src_id,src_port,tgt_id,tgt_port)") → RGBA
    #[serde(default)]
    pub connection_colors: HashMap<String, [f32; 4]>,
}

impl ColorOverrides {
    /// Returns the colour override for a component, if any.
    pub fn get_component_color(&self, comp_id: usize) -> Option<Color> {
        self.component_colors
            .get(&comp_id)
            .map(|c| Color::new(c[0], c[1], c[2], c[3]))
    }

    /// Sets (or clears) a colour override for a component.
    pub fn set_component_color(&mut self, comp_id: usize, color: Option<[f32; 4]>) {
        match color {
            Some(c) => {
                self.component_colors.insert(comp_id, c);
            }
            None => {
                self.component_colors.remove(&comp_id);
            }
        }
    }

    /// Returns the colour override for a wire, if any.
    pub fn get_wire_color(&self, conn: &VisualConnection) -> Option<Color> {
        let key = Self::wire_key(conn);
        self.connection_colors
            .get(&key)
            .map(|c| Color::new(c[0], c[1], c[2], c[3]))
    }

    /// Sets (or clears) a colour override for a wire.
    pub fn set_wire_color(&mut self, conn: &VisualConnection, color: Option<[f32; 4]>) {
        let key = Self::wire_key(conn);
        match color {
            Some(c) => {
                self.connection_colors.insert(key, c);
            }
            None => {
                self.connection_colors.remove(&key);
            }
        }
    }

    fn wire_key(conn: &VisualConnection) -> String {
        format!(
            "{},{},{},{}",
            conn.src_comp_id, conn.src_port, conn.tgt_comp_id, conn.tgt_port
        )
    }
}
