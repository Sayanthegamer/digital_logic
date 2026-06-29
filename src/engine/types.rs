#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GateType {
    Nand,
    Input,
    Output,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrimitiveGate {
    pub gate_type: GateType,
    pub input_a_source: Option<usize>,
    pub input_b_source: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ComponentType {
    Nand,
    Input,
    Output,
    Clock,
    SubChip(usize), // Index of the sub-chip blueprint in the library/registry
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Component {
    pub component_type: ComponentType,
    pub pos: (f32, f32),             // Visual layout position for inspection mode
    pub clock_period: Option<usize>, // Localized period in ticks (only for ComponentType::Clock)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SourcePort {
    ChipInput(usize), // The i-th input of the custom chip itself
    ComponentOutput {
        component_idx: usize,
        port_idx: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TargetPort {
    ChipOutput(usize), // The j-th output of the custom chip itself
    ComponentInput {
        component_idx: usize,
        port_idx: usize,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Connection {
    pub source: SourcePort,
    pub target: TargetPort,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChipBlueprint {
    pub name: String,
    pub inputs: usize,
    pub outputs: usize,
    pub input_names: Vec<String>,
    pub output_names: Vec<String>,
    pub components: Vec<Component>,
    pub connections: Vec<Connection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OutputSource {
    DrivenByGate(usize),  // Driven by a primitive gate index
    PassedThrough(usize), // Driven directly by outer chip input port index
    Floating,             // Not connected internally
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstantiatedInterface {
    pub inputs: Vec<Vec<(usize, u8)>>, // Outer chip input index -> primitive targets (gate_idx, port)
    pub outputs: Vec<OutputSource>,    // Outer chip output index -> its driver
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TraceNode {
    ChipInput(usize),
    ChipOutput(usize),
    CompInput {
        component_idx: usize,
        port_idx: usize,
    },
    CompOutput {
        component_idx: usize,
        port_idx: usize,
    },
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct CompiledClock {
    pub gate_idx: usize,
    pub period: usize,
    pub counter: usize,
    pub visual_id: Option<usize>, // Top-level visual component ID if mapped
}
