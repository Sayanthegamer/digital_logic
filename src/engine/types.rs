#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GateType {
    Nand,
    Input,
    Output,
    TriStateBuffer,
    BusResolver,
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
    SubChip(usize),
    SevenSegment,
    TriStateBuffer,
    Junction,
    BusJoiner,
    BusSplitter,
}

impl ComponentType {
    pub fn get_port_counts(&self, bus_width: Option<usize>, library: &[ChipBlueprint]) -> (usize, usize) {
        match self {
            ComponentType::Nand => (2, 1),
            ComponentType::Input => (0, 1),
            ComponentType::Output => (1, 0),
            ComponentType::Clock => (0, 1),
            ComponentType::SevenSegment => (8, 0),
            ComponentType::TriStateBuffer => (2, 1),
            ComponentType::Junction => (1, 1),
            ComponentType::BusJoiner => (bus_width.unwrap_or(4), 1),
            ComponentType::BusSplitter => (1, bus_width.unwrap_or(4)),
            ComponentType::SubChip(idx) => {
                if let Some(bp) = library.get(*idx) {
                    (bp.inputs, bp.outputs)
                } else {
                    (0, 0)
                }
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Component {
    pub component_type: ComponentType,
    pub pos: (f32, f32),             // Visual layout position for inspection mode
    pub clock_period: Option<usize>, // Localized period in ticks (only for ComponentType::Clock)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bus_width: Option<usize>,    // Width of the bus (only for BusJoiner/BusSplitter)
}

impl Component {
    pub fn bus_width(&self) -> usize {
        self.bus_width.unwrap_or(4)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SourcePort {
    ChipInput(usize), // The i-th input of the custom chip itself
    ComponentOutput {
        component_idx: usize,
        port_idx: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct InstanceTree {
    pub gate_idx: Option<usize>,
    pub sub_instances: std::collections::HashMap<usize, InstanceTree>,
    pub outputs: Vec<OutputSource>,
}

impl InstanceTree {
    pub fn get_instance(&self, path: &[usize], comp_idx: usize) -> Option<&InstanceTree> {
        let mut curr = self;
        for &p in path {
            curr = curr.sub_instances.get(&p)?;
        }
        curr.sub_instances.get(&comp_idx)
    }
}
