use egui::Color32;
use macroquad::color::Color;

// --- Colors ---

pub struct ThemeColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ThemeColor {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn mq(&self) -> Color {
        Color::new(self.r, self.g, self.b, self.a)
    }

    pub fn mq_with_alpha(&self, alpha: f32) -> Color {
        Color::new(self.r, self.g, self.b, alpha)
    }

    pub fn egui(&self) -> Color32 {
        Color32::from_rgba_unmultiplied(
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }
}

// Backgrounds
pub const BG_CANVAS: ThemeColor = ThemeColor::new(0.09, 0.10, 0.12, 1.0); // #171A1F
pub const BG_PANEL: ThemeColor = ThemeColor::new(0.12, 0.13, 0.15, 1.0); // #1F2329
pub const BORDER: ThemeColor = ThemeColor::new(0.20, 0.23, 0.26, 1.0); // #333942

// Text
pub const TEXT_PRIMARY: ThemeColor = ThemeColor::new(0.85, 0.88, 0.90, 1.0); // #D9E0E6
pub const TEXT_SECONDARY: ThemeColor = ThemeColor::new(0.50, 0.55, 0.60, 1.0); // #808C99

// Accents
pub const ACCENT_PRIMARY: ThemeColor = ThemeColor::new(0.00, 0.70, 1.00, 1.0); // #00B2FF
pub const ACCENT_ACTIVE: ThemeColor = ThemeColor::new(0.15, 0.85, 0.40, 1.0); // #26D966
pub const ACCENT_ERROR: ThemeColor = ThemeColor::new(0.90, 0.22, 0.27, 1.0); // #E63946
pub const ACCENT_INACTIVE: ThemeColor = ThemeColor::new(0.24, 0.27, 0.30, 1.0); // #3D454D
pub const ACCENT_GENERIC: ThemeColor = ThemeColor::new(0.40, 0.45, 0.50, 1.0); // #667380

// Component Specific
pub const COMP_NAND: ThemeColor = ThemeColor::new(1.0, 0.55, 0.15, 1.0); // Amber orange
pub const COMP_CLOCK: ThemeColor = ThemeColor::new(0.00, 0.70, 1.00, 1.0); // Electric cyan
pub const COMP_SUBCHIP: ThemeColor = ThemeColor::new(0.40, 0.45, 0.85, 1.0); // Royal indigo
pub const COMP_SEVENSEG: ThemeColor = ThemeColor::new(0.90, 0.20, 0.20, 1.0); // Red

// Spacing & Layout Constants
pub const PADDING_SMALL: f32 = 4.0;
pub const PADDING_BASE: f32 = 8.0;
pub const PADDING_LARGE: f32 = 16.0;

// Icon Unicode Constants (Material Icons)
pub const ICON_PLAY: &str = "\u{e037}"; // play_arrow
pub const ICON_PAUSE: &str = "\u{e034}"; // pause
pub const ICON_STOP: &str = "\u{e047}"; // stop
pub const ICON_DELETE: &str = "\u{e872}"; // delete
pub const ICON_ADD: &str = "\u{e145}"; // add
pub const ICON_UNDO: &str = "\u{e166}"; // undo
pub const ICON_SETTINGS: &str = "\u{e8b8}"; // settings
pub const ICON_SAVE: &str = "\u{e161}"; // save
pub const ICON_INFO: &str = "\u{e88e}"; // info
pub const ICON_WARNING: &str = "\u{e002}"; // warning
pub const ICON_FOLDER: &str = "\u{e2c7}"; // folder
pub const ICON_EDIT: &str = "\u{e3c9}"; // edit
pub const ICON_SEARCH: &str = "\u{e8b6}"; // search
pub const ICON_CLOSE: &str = "\u{e5cd}"; // close
pub const ICON_CLEAR: &str = "\u{e14c}"; // clear
pub const ICON_REDO: &str = "\u{e15a}"; // redo
pub const ICON_RECENTER: &str = "\u{e55c}"; // pin_drop / filter_center_focus
pub const ICON_REFRESH: &str = "\u{e5d5}"; // refresh
pub const ICON_IMAGE: &str = "\u{e3f4}";   // image / export picture

