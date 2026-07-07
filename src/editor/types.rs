use crate::engine::ComponentType;
use macroquad::prelude::*;

pub mod serde_vec2 {
    use macroquad::prelude::Vec2;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(vec: &Vec2, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let tup = (vec.x, vec.y);
        serde::Serialize::serialize(&tup, serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec2, D::Error>
    where
        D: Deserializer<'de>,
    {
        let tup: (f32, f32) = Deserialize::deserialize(deserializer)?;
        Ok(Vec2::new(tup.0, tup.1))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VisualComponent {
    pub id: usize,
    pub comp_type: ComponentType,
    #[serde(with = "serde_vec2")]
    pub pos: Vec2,
    pub width: f32,
    pub height: f32,
    pub label: String,
    pub clock_period: Option<usize>, // Localized period in ticks (only for Clock)
    /// Per-component colour override (RGBA). None = use theme default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[f32; 4]>,
}

impl VisualComponent {
    pub fn bus_width(&self) -> usize {
        self.clock_period.unwrap_or(4)
    }

    pub fn input_port_pos(&self, port_idx: usize, num_inputs: usize) -> Vec2 {
        if self.comp_type == ComponentType::Junction {
            // Treat a Junction as a thin bar (either horizontal or vertical).
            let horizontal = self.width >= self.height;
            if horizontal {
                // input = left end
                return Vec2::new(self.pos.x, self.pos.y + self.height / 2.0);
            }
            // input = top end
            return Vec2::new(self.pos.x + self.width / 2.0, self.pos.y);
        }
        if num_inputs == 0 {
            return self.pos;
        }
        let spacing = self.height / (num_inputs + 1) as f32;
        let y = self.pos.y + spacing * (port_idx + 1) as f32;
        Vec2::new(self.pos.x, y)
    }

    pub fn output_port_pos(&self, port_idx: usize, num_outputs: usize) -> Vec2 {
        if self.comp_type == ComponentType::Junction {
            // Treat a Junction as a thin bar (either horizontal or vertical).
            let horizontal = self.width >= self.height;
            if horizontal {
                // output = right end
                return Vec2::new(self.pos.x + self.width, self.pos.y + self.height / 2.0);
            }
            // output = bottom end
            return Vec2::new(self.pos.x + self.width / 2.0, self.pos.y + self.height);
        }
        if num_outputs == 0 {
            return self.pos + Vec2::new(self.width, 0.0);
        }
        let spacing = self.height / (num_outputs + 1) as f32;
        let y = self.pos.y + spacing * (port_idx + 1) as f32;
        Vec2::new(self.pos.x + self.width, y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Hash)]
pub struct VisualConnection {
    pub src_comp_id: usize,
    pub src_port: usize,
    pub tgt_comp_id: usize,
    pub tgt_port: usize,
}

impl VisualConnection {
    /// Per-wire colour override (RGBA). Stored externally in ColorOverrides.
    /// This method is just documentation for the pattern.
    pub fn color_key(&self) -> String {
        format!(
            "{},{},{},{}",
            self.src_comp_id, self.src_port, self.tgt_comp_id, self.tgt_port
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActiveTool {
    PlaceComponent(ComponentType),
    PlaceAnnotation,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TextAnnotation {
    pub text: String,
    #[serde(with = "serde_vec2")]
    pub pos: Vec2,
}
