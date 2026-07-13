// UI & Rendering
pub const DEFAULT_COMP_HEIGHT: f32 = 40.0;
pub const JUNCTION_SIZE: f32 = 12.0;

// Grid & Snapping
pub const SNAP_GRID_SIZE: f32 = 20.0;
pub const PORT_HOVER_RADIUS: f32 = 25.0;

// Wire Visualization States
pub const WIRE_STATE_FLOATING: u8 = 0b00;
pub const WIRE_STATE_LOW: u8 = 0b01;
pub const WIRE_STATE_HIGH: u8 = 0b10;
pub const WIRE_STATE_CONTENTION: u8 = 0b11;

// Performance limits
pub const WIRE_DISABLE_THRESHOLD: usize = 5000;

// Special IDs
pub const FAKE_OUTPUT_COMP_ID: usize = 8888;
