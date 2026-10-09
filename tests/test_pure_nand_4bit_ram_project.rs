#![allow(clippy::needless_range_loop, clippy::vec_init_then_push)]

use logic_simulator::editor::ProjectFile;
use logic_simulator::editor::types::*;
use logic_simulator::engine::types::*;
use macroquad::prelude::Vec2;

/// Builder for all chip blueprints in the hierarchy
pub struct ChipLibraryBuilder {
    pub library: Vec<ChipBlueprint>,
}

impl Default for ChipLibraryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ChipLibraryBuilder {
    pub fn new() -> Self {
        Self {
            library: Vec::new(),
        }
    }

    pub fn build_all() -> Vec<ChipBlueprint> {
        let mut builder = Self::new();

        // Level 1: Basic Logic Gates (Pure NAND)
        let not_idx = builder.add_not();
        let and_idx = builder.add_and();
        let or_idx = builder.add_or();
        let _nor_idx = builder.add_nor();
        let xor_idx = builder.add_xor();
        let _xnor_idx = builder.add_xnor();

        // Level 2: Arithmetic Adders
        let ha_idx = builder.add_half_adder(xor_idx, and_idx);
        let fa_idx = builder.add_full_adder(ha_idx, or_idx);
        let adder4_idx = builder.add_adder_4bit(fa_idx);

        // Level 3: Multiplexers and Decoders
        let mux2_idx = builder.add_mux_2to1(not_idx, and_idx, or_idx);
        let mux4_idx = builder.add_mux_4to1(mux2_idx);
        let _mux4bit_2to1_idx = builder.add_mux_4bit_2to1(mux2_idx);
        let mux4bit_4to1_idx = builder.add_mux_4bit_4to1(mux4_idx);
        let dec2to4_idx = builder.add_decoder_2to4(not_idx, and_idx);

        // Level 4: Memory & Latches
        let dlatch_idx = builder.add_dlatch();
        let bitcell_idx = builder.add_bitcell(dlatch_idx);

        // Level 5: 4-Bit Word Register
        let reg4_idx = builder.add_register_4bit(bitcell_idx);

        // Level 6: 4-Word x 4-Bit Addressable RAM
        let ram_idx = builder.add_ram_4x4bit(and_idx, dec2to4_idx, reg4_idx, mux4bit_4to1_idx);

        // Level 7: Even Higher Level - RAM + ALU System
        let _alu_idx = builder.add_ram_alu_unit(ram_idx, adder4_idx);

        builder.library
    }

    // --- Level 1 ---

    fn add_not(&mut self) -> usize {
        let idx = self.library.len();
        self.library.push(ChipBlueprint {
            name: "NOT".to_string(),
            inputs: 1,
            outputs: 1,
            input_names: vec!["IN".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![Component {
                component_type: ComponentType::Nand,
                pos: (200.0, 150.0),
                clock_period: None,
                bus_width: None,
            }],
            connections: vec![
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
            ],
        });
        idx
    }

    fn add_and(&mut self) -> usize {
        let idx = self.library.len();
        self.library.push(ChipBlueprint {
            name: "AND".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 150.0),
                    clock_period: None,
                    bus_width: None,
                },
                Component {
                    component_type: ComponentType::Nand,
                    pos: (380.0, 150.0),
                    clock_period: None,
                    bus_width: None,
                },
            ],
            connections: vec![
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
            ],
        });
        idx
    }

    fn add_or(&mut self) -> usize {
        let idx = self.library.len();
        self.library.push(ChipBlueprint {
            name: "OR".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 100.0),
                    clock_period: None,
                    bus_width: None,
                },
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 240.0),
                    clock_period: None,
                    bus_width: None,
                },
                Component {
                    component_type: ComponentType::Nand,
                    pos: (400.0, 170.0),
                    clock_period: None,
                    bus_width: None,
                },
            ],
            connections: vec![
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
            ],
        });
        idx
    }

    fn add_nor(&mut self) -> usize {
        let idx = self.library.len();
        self.library.push(ChipBlueprint {
            name: "NOR".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 100.0),
                    clock_period: None,
                    bus_width: None,
                },
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 240.0),
                    clock_period: None,
                    bus_width: None,
                },
                Component {
                    component_type: ComponentType::Nand,
                    pos: (380.0, 170.0),
                    clock_period: None,
                    bus_width: None,
                },
                Component {
                    component_type: ComponentType::Nand,
                    pos: (540.0, 170.0),
                    clock_period: None,
                    bus_width: None,
                },
            ],
            connections: vec![
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
            ],
        });
        idx
    }

    fn add_xor(&mut self) -> usize {
        let idx = self.library.len();
        self.library.push(ChipBlueprint {
            name: "XOR".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 170.0),
                    clock_period: None,
                    bus_width: None,
                }, // 0
                Component {
                    component_type: ComponentType::Nand,
                    pos: (360.0, 100.0),
                    clock_period: None,
                    bus_width: None,
                }, // 1
                Component {
                    component_type: ComponentType::Nand,
                    pos: (360.0, 240.0),
                    clock_period: None,
                    bus_width: None,
                }, // 2
                Component {
                    component_type: ComponentType::Nand,
                    pos: (520.0, 170.0),
                    clock_period: None,
                    bus_width: None,
                }, // 3
            ],
            connections: vec![
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
            ],
        });
        idx
    }

    fn add_xnor(&mut self) -> usize {
        let idx = self.library.len();
        self.library.push(ChipBlueprint {
            name: "XNOR".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 170.0),
                    clock_period: None,
                    bus_width: None,
                }, // 0
                Component {
                    component_type: ComponentType::Nand,
                    pos: (360.0, 100.0),
                    clock_period: None,
                    bus_width: None,
                }, // 1
                Component {
                    component_type: ComponentType::Nand,
                    pos: (360.0, 240.0),
                    clock_period: None,
                    bus_width: None,
                }, // 2
                Component {
                    component_type: ComponentType::Nand,
                    pos: (520.0, 170.0),
                    clock_period: None,
                    bus_width: None,
                }, // 3 (XOR)
                Component {
                    component_type: ComponentType::Nand,
                    pos: (670.0, 170.0),
                    clock_period: None,
                    bus_width: None,
                }, // 4 (NOT)
            ],
            connections: vec![
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 4,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 4,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 4,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
            ],
        });
        idx
    }

    // --- Level 2: Adders ---

    fn add_half_adder(&mut self, xor_idx: usize, and_idx: usize) -> usize {
        let idx = self.library.len();
        self.library.push(ChipBlueprint {
            name: "HalfAdder".to_string(),
            inputs: 2,
            outputs: 2,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["Sum".to_string(), "Cout".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::SubChip(xor_idx),
                    pos: (250.0, 100.0),
                    clock_period: None,
                    bus_width: None,
                }, // Comp 0: XOR -> Sum
                Component {
                    component_type: ComponentType::SubChip(and_idx),
                    pos: (250.0, 240.0),
                    clock_period: None,
                    bus_width: None,
                }, // Comp 1: AND -> Cout
            ],
            connections: vec![
                // A -> XOR[0], AND[0]
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                },
                // B -> XOR[1], AND[1]
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                },
                // XOR Out -> Sum
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
                // AND Out -> Cout
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(1),
                },
            ],
        });
        idx
    }

    fn add_full_adder(&mut self, ha_idx: usize, or_idx: usize) -> usize {
        let idx = self.library.len();
        self.library.push(ChipBlueprint {
            name: "FullAdder".to_string(),
            inputs: 3,
            outputs: 2,
            input_names: vec!["A".to_string(), "B".to_string(), "Cin".to_string()],
            output_names: vec!["Sum".to_string(), "Cout".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::SubChip(ha_idx),
                    pos: (200.0, 120.0),
                    clock_period: None,
                    bus_width: None,
                }, // Comp 0: HA1(A, B)
                Component {
                    component_type: ComponentType::SubChip(ha_idx),
                    pos: (380.0, 150.0),
                    clock_period: None,
                    bus_width: None,
                }, // Comp 1: HA2(s1, Cin)
                Component {
                    component_type: ComponentType::SubChip(or_idx),
                    pos: (540.0, 240.0),
                    clock_period: None,
                    bus_width: None,
                }, // Comp 2: OR(c1, c2)
            ],
            connections: vec![
                // Inputs to HA 0
                Connection {
                    source: SourcePort::ChipInput(0), // A
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1), // B
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                // HA 0 Sum -> HA 1 input 0
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                },
                // Cin -> HA 1 input 1
                Connection {
                    source: SourcePort::ChipInput(2),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                },
                // HA 1 Sum -> Chip Output 0 (Sum)
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
                // HA 0 Cout -> OR input 0
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                },
                // HA 1 Cout -> OR input 1
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 1,
                    },
                },
                // OR Out -> Chip Output 1 (Cout)
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(1),
                },
            ],
        });
        idx
    }

    fn add_adder_4bit(&mut self, fa_idx: usize) -> usize {
        let idx = self.library.len();
        self.library.push(ChipBlueprint {
            name: "Adder4Bit".to_string(),
            inputs: 9,
            outputs: 5,
            input_names: vec![
                "A0".to_string(),
                "A1".to_string(),
                "A2".to_string(),
                "A3".to_string(),
                "B0".to_string(),
                "B1".to_string(),
                "B2".to_string(),
                "B3".to_string(),
                "Cin".to_string(),
            ],
            output_names: vec![
                "S0".to_string(),
                "S1".to_string(),
                "S2".to_string(),
                "S3".to_string(),
                "Cout".to_string(),
            ],
            components: vec![
                Component {
                    component_type: ComponentType::SubChip(fa_idx),
                    pos: (200.0, 80.0),
                    clock_period: None,
                    bus_width: None,
                }, // FA 0
                Component {
                    component_type: ComponentType::SubChip(fa_idx),
                    pos: (380.0, 80.0),
                    clock_period: None,
                    bus_width: None,
                }, // FA 1
                Component {
                    component_type: ComponentType::SubChip(fa_idx),
                    pos: (560.0, 80.0),
                    clock_period: None,
                    bus_width: None,
                }, // FA 2
                Component {
                    component_type: ComponentType::SubChip(fa_idx),
                    pos: (740.0, 80.0),
                    clock_period: None,
                    bus_width: None,
                }, // FA 3
            ],
            connections: vec![
                // FA 0: A0, B0, Cin
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(4),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(8),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 2,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
                // FA 0 Cout -> FA 1 Cin
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 2,
                    },
                },
                // FA 1: A1, B1
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(5),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(1),
                },
                // FA 1 Cout -> FA 2 Cin
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 2,
                    },
                },
                // FA 2: A2, B2
                Connection {
                    source: SourcePort::ChipInput(2),
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(6),
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(2),
                },
                // FA 2 Cout -> FA 3 Cin
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 2,
                        port_idx: 1,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 2,
                    },
                },
                // FA 3: A3, B3
                Connection {
                    source: SourcePort::ChipInput(3),
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(7),
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(3),
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 3,
                        port_idx: 1,
                    },
                    target: TargetPort::ChipOutput(4),
                },
            ],
        });
        idx
    }

    // --- Level 3: Multiplexers and Decoders ---

    fn add_mux_2to1(&mut self, not_idx: usize, and_idx: usize, or_idx: usize) -> usize {
        let idx = self.library.len();
        self.library.push(ChipBlueprint {
            name: "MUX2to1".to_string(),
            inputs: 3,
            outputs: 1,
            input_names: vec!["D0".to_string(), "D1".to_string(), "SEL".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::SubChip(not_idx),
                    pos: (200.0, 220.0),
                    clock_period: None,
                    bus_width: None,
                }, // Comp 0: NOT(SEL)
                Component {
                    component_type: ComponentType::SubChip(and_idx),
                    pos: (360.0, 100.0),
                    clock_period: None,
                    bus_width: None,
                }, // Comp 1: AND(D0, NOT_SEL)
                Component {
                    component_type: ComponentType::SubChip(and_idx),
                    pos: (360.0, 240.0),
                    clock_period: None,
                    bus_width: None,
                }, // Comp 2: AND(D1, SEL)
                Component {
                    component_type: ComponentType::SubChip(or_idx),
                    pos: (520.0, 170.0),
                    clock_period: None,
                    bus_width: None,
                }, // Comp 3: OR
            ],
            connections: vec![
                // SEL -> NOT(SEL)
                Connection {
                    source: SourcePort::ChipInput(2),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                // D0 -> AND 1[0]
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                },
                // NOT_SEL -> AND 1[1]
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                },
                // D1 -> AND 2[0]
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                },
                // SEL -> AND 2[1]
                Connection {
                    source: SourcePort::ChipInput(2),
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 1,
                    },
                },
                // AND 1 -> OR[0]
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                },
                // AND 2 -> OR[1]
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 1,
                    },
                },
                // OR -> OUT
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
            ],
        });
        idx
    }

    fn add_mux_4to1(&mut self, mux2_idx: usize) -> usize {
        let idx = self.library.len();
        self.library.push(ChipBlueprint {
            name: "MUX4to1".to_string(),
            inputs: 6,
            outputs: 1,
            input_names: vec![
                "D0".to_string(),
                "D1".to_string(),
                "D2".to_string(),
                "D3".to_string(),
                "S0".to_string(),
                "S1".to_string(),
            ],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::SubChip(mux2_idx),
                    pos: (220.0, 100.0),
                    clock_period: None,
                    bus_width: None,
                }, // MUX 0: D0, D1, S0
                Component {
                    component_type: ComponentType::SubChip(mux2_idx),
                    pos: (220.0, 240.0),
                    clock_period: None,
                    bus_width: None,
                }, // MUX 1: D2, D3, S0
                Component {
                    component_type: ComponentType::SubChip(mux2_idx),
                    pos: (440.0, 170.0),
                    clock_period: None,
                    bus_width: None,
                }, // MUX 2: M0, M1, S1
            ],
            connections: vec![
                // MUX 0: D0, D1, S0
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(4), // S0
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 2,
                    },
                },
                // MUX 1: D2, D3, S0
                Connection {
                    source: SourcePort::ChipInput(2),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(3),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(4), // S0
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 2,
                    },
                },
                // MUX 2: MUX0 Out, MUX1 Out, S1
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(5), // S1
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 2,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
            ],
        });
        idx
    }

    fn add_mux_4bit_2to1(&mut self, mux2_idx: usize) -> usize {
        let idx = self.library.len();
        let mut components = Vec::new();
        let mut connections = Vec::new();

        for i in 0..4 {
            components.push(Component {
                component_type: ComponentType::SubChip(mux2_idx),
                pos: (250.0, 70.0 + (i as f32 * 110.0)),
                clock_period: None,
                bus_width: None,
            });
            // A[i] -> MUX[0]
            connections.push(Connection {
                source: SourcePort::ChipInput(i),
                target: TargetPort::ComponentInput {
                    component_idx: i,
                    port_idx: 0,
                },
            });
            // B[i] -> MUX[1]
            connections.push(Connection {
                source: SourcePort::ChipInput(4 + i),
                target: TargetPort::ComponentInput {
                    component_idx: i,
                    port_idx: 1,
                },
            });
            // SEL -> MUX[2]
            connections.push(Connection {
                source: SourcePort::ChipInput(8),
                target: TargetPort::ComponentInput {
                    component_idx: i,
                    port_idx: 2,
                },
            });
            // OUT[i]
            connections.push(Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: i,
                    port_idx: 0,
                },
                target: TargetPort::ChipOutput(i),
            });
        }

        self.library.push(ChipBlueprint {
            name: "MUX4Bit_2to1".to_string(),
            inputs: 9,
            outputs: 4,
            input_names: vec![
                "A0".to_string(),
                "A1".to_string(),
                "A2".to_string(),
                "A3".to_string(),
                "B0".to_string(),
                "B1".to_string(),
                "B2".to_string(),
                "B3".to_string(),
                "SEL".to_string(),
            ],
            output_names: vec![
                "OUT0".to_string(),
                "OUT1".to_string(),
                "OUT2".to_string(),
                "OUT3".to_string(),
            ],
            components,
            connections,
        });
        idx
    }

    fn add_mux_4bit_4to1(&mut self, mux4_idx: usize) -> usize {
        let idx = self.library.len();
        let mut components = Vec::new();
        let mut connections = Vec::new();

        for bit in 0..4 {
            components.push(Component {
                component_type: ComponentType::SubChip(mux4_idx),
                pos: (250.0, 60.0 + (bit as f32 * 140.0)),
                clock_period: None,
                bus_width: None,
            });
            // Word 0 bit -> port 0
            connections.push(Connection {
                source: SourcePort::ChipInput(bit),
                target: TargetPort::ComponentInput {
                    component_idx: bit,
                    port_idx: 0,
                },
            });
            // Word 1 bit -> port 1
            connections.push(Connection {
                source: SourcePort::ChipInput(4 + bit),
                target: TargetPort::ComponentInput {
                    component_idx: bit,
                    port_idx: 1,
                },
            });
            // Word 2 bit -> port 2
            connections.push(Connection {
                source: SourcePort::ChipInput(8 + bit),
                target: TargetPort::ComponentInput {
                    component_idx: bit,
                    port_idx: 2,
                },
            });
            // Word 3 bit -> port 3
            connections.push(Connection {
                source: SourcePort::ChipInput(12 + bit),
                target: TargetPort::ComponentInput {
                    component_idx: bit,
                    port_idx: 3,
                },
            });
            // S0 -> port 4
            connections.push(Connection {
                source: SourcePort::ChipInput(16),
                target: TargetPort::ComponentInput {
                    component_idx: bit,
                    port_idx: 4,
                },
            });
            // S1 -> port 5
            connections.push(Connection {
                source: SourcePort::ChipInput(17),
                target: TargetPort::ComponentInput {
                    component_idx: bit,
                    port_idx: 5,
                },
            });
            // OUT
            connections.push(Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: bit,
                    port_idx: 0,
                },
                target: TargetPort::ChipOutput(bit),
            });
        }

        self.library.push(ChipBlueprint {
            name: "MUX4Bit_4to1".to_string(),
            inputs: 18,
            outputs: 4,
            input_names: vec![
                "W0_0".to_string(),
                "W0_1".to_string(),
                "W0_2".to_string(),
                "W0_3".to_string(),
                "W1_0".to_string(),
                "W1_1".to_string(),
                "W1_2".to_string(),
                "W1_3".to_string(),
                "W2_0".to_string(),
                "W2_1".to_string(),
                "W2_2".to_string(),
                "W2_3".to_string(),
                "W3_0".to_string(),
                "W3_1".to_string(),
                "W3_2".to_string(),
                "W3_3".to_string(),
                "S0".to_string(),
                "S1".to_string(),
            ],
            output_names: vec![
                "OUT0".to_string(),
                "OUT1".to_string(),
                "OUT2".to_string(),
                "OUT3".to_string(),
            ],
            components,
            connections,
        });
        idx
    }

    fn add_decoder_2to4(&mut self, not_idx: usize, and_idx: usize) -> usize {
        let idx = self.library.len();
        // Comp 0: NOT(A0)
        // Comp 1: NOT(A1)
        // Comp 2: AND(not_a1, not_a0)
        // Comp 3: AND(Comp 2, EN) -> Y0
        // Comp 4: AND(not_a1, a0)
        // Comp 5: AND(Comp 4, EN) -> Y1
        // Comp 6: AND(a1, not_a0)
        // Comp 7: AND(Comp 6, EN) -> Y2
        // Comp 8: AND(a1, a0)
        // Comp 9: AND(Comp 8, EN) -> Y3
        let components = vec![
            Component {
                component_type: ComponentType::SubChip(not_idx),
                pos: (200.0, 100.0),
                clock_period: None,
                bus_width: None,
            }, // 0: not_a0
            Component {
                component_type: ComponentType::SubChip(not_idx),
                pos: (200.0, 200.0),
                clock_period: None,
                bus_width: None,
            }, // 1: not_a1
            Component {
                component_type: ComponentType::SubChip(and_idx),
                pos: (380.0, 70.0),
                clock_period: None,
                bus_width: None,
            }, // 2
            Component {
                component_type: ComponentType::SubChip(and_idx),
                pos: (540.0, 70.0),
                clock_period: None,
                bus_width: None,
            }, // 3 (Y0)
            Component {
                component_type: ComponentType::SubChip(and_idx),
                pos: (380.0, 170.0),
                clock_period: None,
                bus_width: None,
            }, // 4
            Component {
                component_type: ComponentType::SubChip(and_idx),
                pos: (540.0, 170.0),
                clock_period: None,
                bus_width: None,
            }, // 5 (Y1)
            Component {
                component_type: ComponentType::SubChip(and_idx),
                pos: (380.0, 270.0),
                clock_period: None,
                bus_width: None,
            }, // 6
            Component {
                component_type: ComponentType::SubChip(and_idx),
                pos: (540.0, 270.0),
                clock_period: None,
                bus_width: None,
            }, // 7 (Y2)
            Component {
                component_type: ComponentType::SubChip(and_idx),
                pos: (380.0, 370.0),
                clock_period: None,
                bus_width: None,
            }, // 8
            Component {
                component_type: ComponentType::SubChip(and_idx),
                pos: (540.0, 370.0),
                clock_period: None,
                bus_width: None,
            }, // 9 (Y3)
        ];

        let connections = vec![
            // A0 -> NOT 0
            Connection {
                source: SourcePort::ChipInput(0),
                target: TargetPort::ComponentInput {
                    component_idx: 0,
                    port_idx: 0,
                },
            },
            // A1 -> NOT 1
            Connection {
                source: SourcePort::ChipInput(1),
                target: TargetPort::ComponentInput {
                    component_idx: 1,
                    port_idx: 0,
                },
            },
            // Line 0: not_a1 (comp 1) & not_a0 (comp 0)
            Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 1,
                    port_idx: 0,
                },
                target: TargetPort::ComponentInput {
                    component_idx: 2,
                    port_idx: 0,
                },
            },
            Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 0,
                    port_idx: 0,
                },
                target: TargetPort::ComponentInput {
                    component_idx: 2,
                    port_idx: 1,
                },
            },
            Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 2,
                    port_idx: 0,
                },
                target: TargetPort::ComponentInput {
                    component_idx: 3,
                    port_idx: 0,
                },
            },
            Connection {
                source: SourcePort::ChipInput(2), // EN
                target: TargetPort::ComponentInput {
                    component_idx: 3,
                    port_idx: 1,
                },
            },
            Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 3,
                    port_idx: 0,
                },
                target: TargetPort::ChipOutput(0),
            },
            // Line 1: not_a1 (comp 1) & a0 (input 0)
            Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 1,
                    port_idx: 0,
                },
                target: TargetPort::ComponentInput {
                    component_idx: 4,
                    port_idx: 0,
                },
            },
            Connection {
                source: SourcePort::ChipInput(0),
                target: TargetPort::ComponentInput {
                    component_idx: 4,
                    port_idx: 1,
                },
            },
            Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 4,
                    port_idx: 0,
                },
                target: TargetPort::ComponentInput {
                    component_idx: 5,
                    port_idx: 0,
                },
            },
            Connection {
                source: SourcePort::ChipInput(2), // EN
                target: TargetPort::ComponentInput {
                    component_idx: 5,
                    port_idx: 1,
                },
            },
            Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 5,
                    port_idx: 0,
                },
                target: TargetPort::ChipOutput(1),
            },
            // Line 2: a1 (input 1) & not_a0 (comp 0)
            Connection {
                source: SourcePort::ChipInput(1),
                target: TargetPort::ComponentInput {
                    component_idx: 6,
                    port_idx: 0,
                },
            },
            Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 0,
                    port_idx: 0,
                },
                target: TargetPort::ComponentInput {
                    component_idx: 6,
                    port_idx: 1,
                },
            },
            Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 6,
                    port_idx: 0,
                },
                target: TargetPort::ComponentInput {
                    component_idx: 7,
                    port_idx: 0,
                },
            },
            Connection {
                source: SourcePort::ChipInput(2), // EN
                target: TargetPort::ComponentInput {
                    component_idx: 7,
                    port_idx: 1,
                },
            },
            Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 7,
                    port_idx: 0,
                },
                target: TargetPort::ChipOutput(2),
            },
            // Line 3: a1 (input 1) & a0 (input 0)
            Connection {
                source: SourcePort::ChipInput(1),
                target: TargetPort::ComponentInput {
                    component_idx: 8,
                    port_idx: 0,
                },
            },
            Connection {
                source: SourcePort::ChipInput(0),
                target: TargetPort::ComponentInput {
                    component_idx: 8,
                    port_idx: 1,
                },
            },
            Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 8,
                    port_idx: 0,
                },
                target: TargetPort::ComponentInput {
                    component_idx: 9,
                    port_idx: 0,
                },
            },
            Connection {
                source: SourcePort::ChipInput(2), // EN
                target: TargetPort::ComponentInput {
                    component_idx: 9,
                    port_idx: 1,
                },
            },
            Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 9,
                    port_idx: 0,
                },
                target: TargetPort::ChipOutput(3),
            },
        ];

        self.library.push(ChipBlueprint {
            name: "Decoder2to4".to_string(),
            inputs: 3,
            outputs: 4,
            input_names: vec!["A0".to_string(), "A1".to_string(), "Strobe".to_string()],
            output_names: vec![
                "Y0".to_string(),
                "Y1".to_string(),
                "Y2".to_string(),
                "Y3".to_string(),
            ],
            components,
            connections,
        });
        idx
    }

    // --- Level 4: Latches & Bit Cell ---

    fn add_dlatch(&mut self) -> usize {
        let idx = self.library.len();
        self.library.push(ChipBlueprint {
            name: "DLatch".to_string(),
            inputs: 2,
            outputs: 2,
            input_names: vec!["D".to_string(), "Enable".to_string()],
            output_names: vec!["Q".to_string(), "Q_bar".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 100.0),
                    clock_period: None,
                    bus_width: None,
                }, // 0
                Component {
                    component_type: ComponentType::Nand,
                    pos: (350.0, 200.0),
                    clock_period: None,
                    bus_width: None,
                }, // 1
                Component {
                    component_type: ComponentType::Nand,
                    pos: (500.0, 100.0),
                    clock_period: None,
                    bus_width: None,
                }, // 2: Q
                Component {
                    component_type: ComponentType::Nand,
                    pos: (500.0, 200.0),
                    clock_period: None,
                    bus_width: None,
                }, // 3: Q_bar
            ],
            connections: vec![
                // NAND 0: D(0) & EN(1)
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                // NAND 1: NAND0 & EN(1)
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                },
                // NAND 2 (Q): NAND0 & NAND3(Q_bar)
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 1,
                    },
                },
                // NAND 3 (Q_bar): NAND1 & NAND2(Q)
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 1,
                    },
                },
                // Outputs
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(1),
                },
            ],
        });
        idx
    }

    fn add_bitcell(&mut self, dlatch_idx: usize) -> usize {
        let idx = self.library.len();
        self.library.push(ChipBlueprint {
            name: "BitCell".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["Data_In".to_string(), "Write_Enable".to_string()],
            output_names: vec!["Data_Out".to_string()],
            components: vec![Component {
                component_type: ComponentType::SubChip(dlatch_idx),
                pos: (250.0, 150.0),
                clock_period: None,
                bus_width: None,
            }],
            connections: vec![
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
            ],
        });
        idx
    }

    // --- Level 5: 4-Bit Register ---

    fn add_register_4bit(&mut self, bitcell_idx: usize) -> usize {
        let idx = self.library.len();
        let mut components = Vec::new();
        let mut connections = Vec::new();

        for i in 0..4 {
            components.push(Component {
                component_type: ComponentType::SubChip(bitcell_idx),
                pos: (250.0, 60.0 + (i as f32 * 100.0)),
                clock_period: None,
                bus_width: None,
            });
            // Data_In[i]
            connections.push(Connection {
                source: SourcePort::ChipInput(i),
                target: TargetPort::ComponentInput {
                    component_idx: i,
                    port_idx: 0,
                },
            });
            // Write_Enable (shared pin 4)
            connections.push(Connection {
                source: SourcePort::ChipInput(4),
                target: TargetPort::ComponentInput {
                    component_idx: i,
                    port_idx: 1,
                },
            });
            // Data_Out[i]
            connections.push(Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: i,
                    port_idx: 0,
                },
                target: TargetPort::ChipOutput(i),
            });
        }

        self.library.push(ChipBlueprint {
            name: "Register4Bit".to_string(),
            inputs: 5,
            outputs: 4,
            input_names: vec![
                "D0".to_string(),
                "D1".to_string(),
                "D2".to_string(),
                "D3".to_string(),
                "Write_Enable".to_string(),
            ],
            output_names: vec![
                "Q0".to_string(),
                "Q1".to_string(),
                "Q2".to_string(),
                "Q3".to_string(),
            ],
            components,
            connections,
        });
        idx
    }

    // --- Level 6: 4-Word x 4-Bit RAM ---

    fn add_ram_4x4bit(
        &mut self,
        and_idx: usize,
        dec_idx: usize,
        reg_idx: usize,
        mux4bit_4to1_idx: usize,
    ) -> usize {
        let idx = self.library.len();
        // Comp 0: AND(CS, WE) -> Write Pulse
        // Comp 1: Decoder2to4 (Addr0, Addr1, Write Pulse)
        // Comp 2: Register4Bit (Word 0)
        // Comp 3: Register4Bit (Word 1)
        // Comp 4: Register4Bit (Word 2)
        // Comp 5: Register4Bit (Word 3)
        // Comp 6: MUX4Bit_4to1 (Word0, Word1, Word2, Word3, Addr0, Addr1)
        let components = vec![
            Component {
                component_type: ComponentType::SubChip(and_idx),
                pos: (150.0, 50.0),
                clock_period: None,
                bus_width: None,
            }, // 0: AND(CS, WE)
            Component {
                component_type: ComponentType::SubChip(dec_idx),
                pos: (300.0, 100.0),
                clock_period: None,
                bus_width: None,
            }, // 1: Decoder
            Component {
                component_type: ComponentType::SubChip(reg_idx),
                pos: (500.0, 50.0),
                clock_period: None,
                bus_width: None,
            }, // 2: Reg 0
            Component {
                component_type: ComponentType::SubChip(reg_idx),
                pos: (500.0, 180.0),
                clock_period: None,
                bus_width: None,
            }, // 3: Reg 1
            Component {
                component_type: ComponentType::SubChip(reg_idx),
                pos: (500.0, 310.0),
                clock_period: None,
                bus_width: None,
            }, // 4: Reg 2
            Component {
                component_type: ComponentType::SubChip(reg_idx),
                pos: (500.0, 440.0),
                clock_period: None,
                bus_width: None,
            }, // 5: Reg 3
            Component {
                component_type: ComponentType::SubChip(mux4bit_4to1_idx),
                pos: (750.0, 200.0),
                clock_period: None,
                bus_width: None,
            }, // 6: MUX4Bit_4to1
        ];

        let mut connections = Vec::new();

        // CS (in 2) & WE (in 3) -> AND 0
        connections.push(Connection {
            source: SourcePort::ChipInput(2), // CS
            target: TargetPort::ComponentInput {
                component_idx: 0,
                port_idx: 0,
            },
        });
        connections.push(Connection {
            source: SourcePort::ChipInput(3), // WE
            target: TargetPort::ComponentInput {
                component_idx: 0,
                port_idx: 1,
            },
        });

        // Addr0, Addr1 to Decoder (comp 1, ports 0, 1)
        connections.push(Connection {
            source: SourcePort::ChipInput(0),
            target: TargetPort::ComponentInput {
                component_idx: 1,
                port_idx: 0,
            },
        });
        connections.push(Connection {
            source: SourcePort::ChipInput(1),
            target: TargetPort::ComponentInput {
                component_idx: 1,
                port_idx: 1,
            },
        });
        // AND 0 Out -> Decoder Enable (comp 1, port 2)
        connections.push(Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 0,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 1,
                port_idx: 2,
            },
        });

        // Decoder lines Y0..Y3 to Reg 0..3 Write_Enable (port 4)
        for w in 0..4 {
            connections.push(Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 1,
                    port_idx: w,
                },
                target: TargetPort::ComponentInput {
                    component_idx: 2 + w,
                    port_idx: 4,
                },
            });
        }

        // Data Inputs D0..D3 (Chip inputs 4..7) to Reg 0..3 Data_In (ports 0..3)
        for w in 0..4 {
            for bit in 0..4 {
                connections.push(Connection {
                    source: SourcePort::ChipInput(4 + bit),
                    target: TargetPort::ComponentInput {
                        component_idx: 2 + w,
                        port_idx: bit,
                    },
                });
            }
        }

        // Register outputs to MUX4Bit_4to1 inputs (ports 0..15)
        for w in 0..4 {
            for bit in 0..4 {
                connections.push(Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 2 + w,
                        port_idx: bit,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 6,
                        port_idx: (w * 4) + bit,
                    },
                });
            }
        }

        // Connect Addr0, Addr1 to MUX select (ports 16, 17)
        connections.push(Connection {
            source: SourcePort::ChipInput(0), // Addr0
            target: TargetPort::ComponentInput {
                component_idx: 6,
                port_idx: 16,
            },
        });
        connections.push(Connection {
            source: SourcePort::ChipInput(1), // Addr1
            target: TargetPort::ComponentInput {
                component_idx: 6,
                port_idx: 17,
            },
        });

        // MUX outputs to Chip Outputs 0..3
        for bit in 0..4 {
            connections.push(Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 6,
                    port_idx: bit,
                },
                target: TargetPort::ChipOutput(bit),
            });
        }

        self.library.push(ChipBlueprint {
            name: "RAM_4x4Bit".to_string(),
            inputs: 8,
            outputs: 4,
            input_names: vec![
                "Addr0".to_string(),
                "Addr1".to_string(),
                "CS".to_string(),
                "WE".to_string(),
                "Data_In_0".to_string(),
                "Data_In_1".to_string(),
                "Data_In_2".to_string(),
                "Data_In_3".to_string(),
            ],
            output_names: vec![
                "Data_Out_0".to_string(),
                "Data_Out_1".to_string(),
                "Data_Out_2".to_string(),
                "Data_Out_3".to_string(),
            ],
            components,
            connections,
        });
        idx
    }

    // --- Level 7: RAM + ALU System ---

    fn add_ram_alu_unit(&mut self, ram_idx: usize, adder4_idx: usize) -> usize {
        let idx = self.library.len();
        // Comp 0: RAM_4x4Bit
        // Comp 1: Adder4Bit (A = RAM_Out, B = Operand, Cin = Cin)
        let components = vec![
            Component {
                component_type: ComponentType::SubChip(ram_idx),
                pos: (240.0, 150.0),
                clock_period: None,
                bus_width: None,
            },
            Component {
                component_type: ComponentType::SubChip(adder4_idx),
                pos: (540.0, 150.0),
                clock_period: None,
                bus_width: None,
            },
        ];

        let mut connections = Vec::new();

        // Pass Addr0, Addr1, CS, WE to RAM
        for i in 0..4 {
            connections.push(Connection {
                source: SourcePort::ChipInput(i),
                target: TargetPort::ComponentInput {
                    component_idx: 0,
                    port_idx: i,
                },
            });
        }

        // Pass Operand 0..3 to RAM Data_In (ports 4..7)
        for i in 0..4 {
            connections.push(Connection {
                source: SourcePort::ChipInput(4 + i),
                target: TargetPort::ComponentInput {
                    component_idx: 0,
                    port_idx: 4 + i,
                },
            });
        }

        // Pass Operand 0..3 to Adder B inputs (ports 4..7)
        for i in 0..4 {
            connections.push(Connection {
                source: SourcePort::ChipInput(4 + i),
                target: TargetPort::ComponentInput {
                    component_idx: 1,
                    port_idx: 4 + i,
                },
            });
        }

        // Cin to Adder Cin (port 8)
        connections.push(Connection {
            source: SourcePort::ChipInput(8),
            target: TargetPort::ComponentInput {
                component_idx: 1,
                port_idx: 8,
            },
        });

        // RAM Outputs to Chip Outputs (0..3) AND Adder A inputs (ports 0..3)
        for i in 0..4 {
            connections.push(Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 0,
                    port_idx: i,
                },
                target: TargetPort::ChipOutput(i),
            });
            connections.push(Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 0,
                    port_idx: i,
                },
                target: TargetPort::ComponentInput {
                    component_idx: 1,
                    port_idx: i,
                },
            });
        }

        // Adder Sum to Chip Outputs (4..7)
        for i in 0..4 {
            connections.push(Connection {
                source: SourcePort::ComponentOutput {
                    component_idx: 1,
                    port_idx: i,
                },
                target: TargetPort::ChipOutput(4 + i),
            });
        }

        // Adder Cout to Chip Output (8)
        connections.push(Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 1,
                port_idx: 4,
            },
            target: TargetPort::ChipOutput(8),
        });

        self.library.push(ChipBlueprint {
            name: "RAM4Bit_ALU_Unit".to_string(),
            inputs: 9,
            outputs: 9,
            input_names: vec![
                "Addr0".to_string(),
                "Addr1".to_string(),
                "CS".to_string(),
                "WE".to_string(),
                "Operand_0".to_string(),
                "Operand_1".to_string(),
                "Operand_2".to_string(),
                "Operand_3".to_string(),
                "Cin".to_string(),
            ],
            output_names: vec![
                "RAM_Out_0".to_string(),
                "RAM_Out_1".to_string(),
                "RAM_Out_2".to_string(),
                "RAM_Out_3".to_string(),
                "Sum_0".to_string(),
                "Sum_1".to_string(),
                "Sum_2".to_string(),
                "Sum_3".to_string(),
                "Cout".to_string(),
            ],
            components,
            connections,
        });
        idx
    }
}

#[test]
fn test_pure_nand_hierarchy_and_ram_functionality() {
    let library = ChipLibraryBuilder::build_all();

    // Verify all 17 chips were constructed
    assert_eq!(library.len(), 19);
    assert_eq!(library[0].name, "NOT");
    assert_eq!(library[1].name, "AND");
    assert_eq!(library[2].name, "OR");
    assert_eq!(library[3].name, "NOR");
    assert_eq!(library[4].name, "XOR");
    assert_eq!(library[5].name, "XNOR");
    assert_eq!(library[6].name, "HalfAdder");
    assert_eq!(library[7].name, "FullAdder");
    assert_eq!(library[8].name, "Adder4Bit");
    assert_eq!(library[9].name, "MUX2to1");
    assert_eq!(library[10].name, "MUX4to1");
    assert_eq!(library[11].name, "MUX4Bit_2to1");
    assert_eq!(library[12].name, "MUX4Bit_4to1");
    assert_eq!(library[13].name, "Decoder2to4");
    assert_eq!(library[14].name, "DLatch");
    assert_eq!(library[15].name, "BitCell");
    assert_eq!(library[16].name, "Register4Bit");
    assert_eq!(library[17].name, "RAM_4x4Bit");
    assert_eq!(library[18].name, "RAM4Bit_ALU_Unit");

    let mut clocks = Vec::new();
    let mut stack = Vec::new();

    // Unit test MUX2to1
    let mux2_idx = 9;
    let mut sim_m2 = logic_simulator::engine::Simulator::new();
    let (iface_m2, _) = sim_m2
        .instantiate_chip_with_mapping(mux2_idx, &library, &mut clocks, &mut stack)
        .unwrap();
    let d0_in = sim_m2.add_gate(GateType::Input);
    let d1_in = sim_m2.add_gate(GateType::Input);
    let sel_in = sim_m2.add_gate(GateType::Input);
    for &(g, p) in &iface_m2.inputs[0] {
        sim_m2.connect(d0_in, g, p);
    }
    for &(g, p) in &iface_m2.inputs[1] {
        sim_m2.connect(d1_in, g, p);
    }
    for &(g, p) in &iface_m2.inputs[2] {
        sim_m2.connect(sel_in, g, p);
    }
    let out_m2 = match iface_m2.outputs[0] {
        OutputSource::DrivenByGate(g) => g,
        _ => panic!(),
    };
    sim_m2.calculate_depths();
    sim_m2.settle().unwrap();
    // d0=1, d1=0, sel=0 -> out=1
    sim_m2.set_input(d0_in, true);
    sim_m2.set_input(d1_in, false);
    sim_m2.set_input(sel_in, false);
    sim_m2.propagate_events(50).unwrap();
    assert!(sim_m2.get_state(out_m2), "MUX2to1 sel=0 failed");
    // sel=1 -> out=0
    sim_m2.set_input(sel_in, true);
    sim_m2.propagate_events(50).unwrap();
    assert!(!sim_m2.get_state(out_m2), "MUX2to1 sel=1 failed");

    // Unit test Decoder2to4
    let dec_idx = 13;
    let mut sim_dec = logic_simulator::engine::Simulator::new();
    let (iface_dec, _) = sim_dec
        .instantiate_chip_with_mapping(dec_idx, &library, &mut clocks, &mut stack)
        .unwrap();
    let dec_a0 = sim_dec.add_gate(GateType::Input);
    let dec_a1 = sim_dec.add_gate(GateType::Input);
    let dec_en = sim_dec.add_gate(GateType::Input);
    for &(g, p) in &iface_dec.inputs[0] {
        sim_dec.connect(dec_a0, g, p);
    }
    for &(g, p) in &iface_dec.inputs[1] {
        sim_dec.connect(dec_a1, g, p);
    }
    for &(g, p) in &iface_dec.inputs[2] {
        sim_dec.connect(dec_en, g, p);
    }
    let mut dec_outs = Vec::new();
    for i in 0..4 {
        match iface_dec.outputs[i] {
            OutputSource::DrivenByGate(g) => dec_outs.push(g),
            _ => panic!(),
        };
    }
    sim_dec.calculate_depths();
    sim_dec.settle().unwrap();
    sim_dec.set_input(dec_en, true);
    // test 00 -> Y0
    sim_dec.set_input(dec_a0, false);
    sim_dec.set_input(dec_a1, false);
    sim_dec.propagate_events(50).unwrap();
    assert!(sim_dec.get_state(dec_outs[0]), "Decoder 00 Y0 failed");
    assert!(!sim_dec.get_state(dec_outs[1]), "Decoder 00 Y1 failed");
    // test 01 -> Y1
    sim_dec.set_input(dec_a0, true);
    sim_dec.set_input(dec_a1, false);
    sim_dec.propagate_events(50).unwrap();
    assert!(!sim_dec.get_state(dec_outs[0]), "Decoder 01 Y0 failed");
    assert!(sim_dec.get_state(dec_outs[1]), "Decoder 01 Y1 failed");

    let ram_idx = 17;
    let mut sim = logic_simulator::engine::Simulator::new();
    let (interface, _tree) = sim
        .instantiate_chip_with_mapping(ram_idx, &library, &mut clocks, &mut stack)
        .expect("RAM instantiation failed");

    assert_eq!(interface.inputs.len(), 8); // Addr0, Addr1, CS, WE, D0, D1, D2, D3
    assert_eq!(interface.outputs.len(), 4); // Out0, Out1, Out2, Out3

    // Create external inputs
    let mut sim_inputs = Vec::new();
    for i in 0..8 {
        let in_gate = sim.add_gate(GateType::Input);
        for &(tgt_g, p) in &interface.inputs[i] {
            sim.connect(in_gate, tgt_g, p);
        }
        sim_inputs.push(in_gate);
    }

    let mut sim_outputs = Vec::new();
    for i in 0..4 {
        match interface.outputs[i] {
            OutputSource::DrivenByGate(g) => sim_outputs.push(g),
            _ => panic!("Expected output port {i} to be driven by a gate"),
        }
    }

    sim.calculate_depths();
    let old_to_new = sim.defragment_and_sort_by_depth();

    for g in &mut sim_inputs {
        *g = old_to_new[*g];
    }
    for g in &mut sim_outputs {
        *g = old_to_new[*g];
    }

    sim.settle().expect("Initial settle failed");

    let a0 = sim_inputs[0];
    let a1 = sim_inputs[1];
    let cs = sim_inputs[2];
    let we = sim_inputs[3];
    let d = [sim_inputs[4], sim_inputs[5], sim_inputs[6], sim_inputs[7]];

    // Enable Chip Select for normal RAM operations
    sim.set_input(cs, true);

    let write_word = |sim: &mut logic_simulator::engine::Simulator, addr: usize, val: u8| {
        sim.set_input(cs, true);
        sim.set_input(a0, (addr & 1) != 0);
        sim.set_input(a1, (addr & 2) != 0);
        for b in 0..4 {
            sim.set_input(d[b], (val & (1 << b)) != 0);
        }
        // Address setup time (allow decoder to stabilize before pulsing WE)
        sim.propagate_events(100)
            .expect("Address setup propagation failed");

        // Pulse WE High then Low
        sim.set_input(we, true);
        sim.propagate_events(100)
            .expect("Write High propagation failed");
        sim.set_input(we, false);
        sim.propagate_events(100)
            .expect("Write Low propagation failed");
    };

    let read_word = |sim: &mut logic_simulator::engine::Simulator, addr: usize| -> u8 {
        sim.set_input(cs, true);
        sim.set_input(a0, (addr & 1) != 0);
        sim.set_input(a1, (addr & 2) != 0);
        sim.set_input(we, false);
        sim.propagate_events(100).expect("Read propagation failed");
        let mut val = 0u8;
        for b in 0..4 {
            if sim.get_state(sim_outputs[b]) {
                val |= 1 << b;
            }
        }
        val
    };

    // Write values to 4 different addresses in the 4-bit RAM:
    write_word(&mut sim, 0, 0b1010); // 10
    write_word(&mut sim, 1, 0b0101); // 5
    write_word(&mut sim, 2, 0b1111); // 15
    write_word(&mut sim, 3, 0b0011); // 3

    println!("Read 0: {}", read_word(&mut sim, 0));
    println!("Read 1: {}", read_word(&mut sim, 1));
    println!("Read 2: {}", read_word(&mut sim, 2));
    println!("Read 3: {}", read_word(&mut sim, 3));

    // Read back and verify all addresses retained their data!
    assert_eq!(read_word(&mut sim, 0), 0b1010);
    assert_eq!(read_word(&mut sim, 1), 0b0101);
    assert_eq!(read_word(&mut sim, 2), 0b1111);
    assert_eq!(read_word(&mut sim, 3), 0b0011);

    // Overwrite address 1
    write_word(&mut sim, 1, 0b1100); // 12
    assert_eq!(read_word(&mut sim, 1), 0b1100);

    // Verify other addresses were NOT altered!
    assert_eq!(read_word(&mut sim, 0), 0b1010);
    assert_eq!(read_word(&mut sim, 2), 0b1111);
    assert_eq!(read_word(&mut sim, 3), 0b0011);

    // Now test the even higher-level RAM4Bit_ALU_Unit!
    let alu_idx = 18;
    let mut sim_alu = logic_simulator::engine::Simulator::new();
    let mut clocks = Vec::new();
    let mut stack = Vec::new();

    let (interface_alu, _) = sim_alu
        .instantiate_chip_with_mapping(alu_idx, &library, &mut clocks, &mut stack)
        .expect("ALU instantiation failed");

    // Inputs: Addr0, Addr1, CS, WE, Operand0..3, Cin (9 inputs total)
    assert_eq!(interface_alu.inputs.len(), 9);
    // Outputs: RAM_Out0..3, Sum0..3, Cout (9 outputs total)
    assert_eq!(interface_alu.outputs.len(), 9);

    let mut alu_inputs = Vec::new();
    for i in 0..9 {
        let g = sim_alu.add_gate(GateType::Input);
        for &(tgt_g, p) in &interface_alu.inputs[i] {
            sim_alu.connect(g, tgt_g, p);
        }
        alu_inputs.push(g);
    }

    let mut alu_outputs = Vec::new();
    for i in 0..9 {
        match interface_alu.outputs[i] {
            OutputSource::DrivenByGate(g) => alu_outputs.push(g),
            _ => panic!("Expected output port {i} to be driven by a gate"),
        }
    }

    sim_alu.calculate_depths();
    let old_to_new = sim_alu.defragment_and_sort_by_depth();

    for g in &mut alu_inputs {
        *g = old_to_new[*g];
    }
    for g in &mut alu_outputs {
        *g = old_to_new[*g];
    }

    sim_alu.settle().expect("Initial ALU settle failed");

    // Enable CS (alu_inputs[2])
    sim_alu.set_input(alu_inputs[2], true);

    // Write 7 (0b0111) to Address 2
    let write_alu_ram = |sim: &mut logic_simulator::engine::Simulator, addr: usize, val: u8| {
        sim.set_input(alu_inputs[0], (addr & 1) != 0); // Addr0
        sim.set_input(alu_inputs[1], (addr & 2) != 0); // Addr1
        sim.set_input(alu_inputs[2], true); // CS = true
        for b in 0..4 {
            sim.set_input(alu_inputs[4 + b], (val & (1 << b)) != 0); // Operand / DataIn
        }
        sim.set_input(alu_inputs[8], false); // Cin
        sim.propagate_events(100)
            .expect("Address setup propagation failed");

        sim.set_input(alu_inputs[3], true); // WE High
        sim.propagate_events(100)
            .expect("WE High propagation failed");

        sim.set_input(alu_inputs[3], false); // WE Low
        sim.propagate_events(100)
            .expect("WE Low propagation failed");
    };

    write_alu_ram(&mut sim_alu, 2, 7);

    // Read address 0:
    sim_alu.set_input(alu_inputs[0], false);
    sim_alu.set_input(alu_inputs[1], false);
    sim_alu.set_input(alu_inputs[2], true);
    sim_alu.set_input(alu_inputs[3], false);
    sim_alu.propagate_events(100).expect("Propagate Addr 0");
    let mut r0 = 0u8;
    for b in 0..4 {
        if sim_alu.get_state(alu_outputs[b]) {
            r0 |= 1 << b;
        }
    }
    println!("ALU RAM Read Addr 0: {}", r0);

    // Read address 2:
    sim_alu.set_input(alu_inputs[0], false); // Addr0 = 0
    sim_alu.set_input(alu_inputs[1], true); // Addr1 = 1 -> Addr = 2
    sim_alu.set_input(alu_inputs[2], true); // CS = 1
    sim_alu.set_input(alu_inputs[3], false); // WE = 0
    for b in 0..4 {
        sim_alu.set_input(alu_inputs[4 + b], (5 & (1 << b)) != 0);
    }
    sim_alu.set_input(alu_inputs[8], false); // Cin = 0
    sim_alu.propagate_events(200).expect("Propagate ALU");
    let mut r2 = 0u8;
    for b in 0..4 {
        if sim_alu.get_state(alu_outputs[b]) {
            r2 |= 1 << b;
        }
    }
    println!("ALU RAM Read Addr 2: {}", r2);

    // Check RAM Out = 7
    let mut ram_out = 0u8;
    for b in 0..4 {
        if sim_alu.get_state(alu_outputs[b]) {
            ram_out |= 1 << b;
        }
    }
    assert_eq!(ram_out, 7);

    // Check Sum = 7 + 5 = 12 (0b1100), Cout = 0
    let mut sum_out = 0u8;
    for b in 0..4 {
        if sim_alu.get_state(alu_outputs[4 + b]) {
            sum_out |= 1 << b;
        }
    }
    let cout = sim_alu.get_state(alu_outputs[8]);
    assert_eq!(sum_out, 12);
    assert!(!cout);

    // Now toggle Cin = 1 -> Sum = 7 + 5 + 1 = 13 (0b1101)
    sim_alu.set_input(alu_inputs[8], true);
    sim_alu.propagate_events(200).expect("Propagate ALU Cin=1");
    let mut sum_out_cin = 0u8;
    for b in 0..4 {
        if sim_alu.get_state(alu_outputs[4 + b]) {
            sum_out_cin |= 1 << b;
        }
    }
    assert_eq!(sum_out_cin, 13);
    assert!(!sim_alu.get_state(alu_outputs[8]));

    // Now set Operand = 10 (0b1010), Cin = 0 -> 7 + 10 = 17 = 16 + 1 -> Sum = 1, Cout = true!
    for b in 0..4 {
        sim_alu.set_input(alu_inputs[4 + b], (10 & (1 << b)) != 0);
    }
    sim_alu.set_input(alu_inputs[8], false);
    sim_alu
        .propagate_events(200)
        .expect("Propagate ALU Overflow");
    let mut sum_overflow = 0u8;
    for b in 0..4 {
        if sim_alu.get_state(alu_outputs[4 + b]) {
            sum_overflow |= 1 << b;
        }
    }
    let cout_overflow = sim_alu.get_state(alu_outputs[8]);
    assert_eq!(sum_overflow, 1);
    assert!(cout_overflow);
}

pub fn create_pure_nand_ram_project() -> ProjectFile {
    let library = ChipLibraryBuilder::build_all();
    let mut components = Vec::new();
    let mut connections = Vec::new();
    let mut annotations = Vec::new();

    // Top Header Annotation
    annotations.push(TextAnnotation {
        text: "=================================================================\n  PURE NAND 4-BIT RAM & ALU COMPUTER ARCHITECTURE\n=================================================================\nBuilt strictly from first principles starting from single NAND gates:\n- Level 1: Basic Logic (NOT, AND, OR, NOR, XOR, XNOR)\n- Level 2: Arithmetic Adders (Half Adder, Full Adder, 4-Bit Adder)\n- Level 3: Multiplexers & Decoders (2:1 MUX, 4:1 MUX, 4-Bit Bus MUX, 2-to-4 Decoder)\n- Level 4: Latches & BitCells (4-NAND D-Latch, Gated BitCell)\n- Level 5: Registers (4-Bit Word Register)\n- Level 6: Addressable RAM (4 words x 4-bit = 16-bit RAM with CS & WE)\n- Level 7: Integrated System (4-Bit RAM + ALU Accumulator Unit)\n\nTip: Select any subchip and click 'Look Inside' to drill down through all 7 layers down to raw NAND!".to_string(),
        pos: Vec2::new(100.0, 50.0),
    });

    // --- MODULE 1: 4-Word x 4-Bit RAM ---
    annotations.push(TextAnnotation {
        text: "=== MODULE 1: 4-WORD x 4-BIT RAM (Level 6) ===\nHow to Operate:\n1. Select Address using Addr0, Addr1 switches (00=Word0, 01=Word1, 10=Word2, 11=Word3).\n2. Turn CS (Chip Select) ON.\n3. WRITE: Set Data In [D0..D3], then click WE (Write Enable) ON then OFF.\n4. READ: Keep WE OFF and CS ON. The LEDs [Q0..Q3] immediately display the stored 4-bit word!".to_string(),
        pos: Vec2::new(100.0, 260.0),
    });

    // Inputs:
    // Addr0 (ID 1), Addr1 (ID 2), CS (ID 3), WE (ID 4), D0..D3 (IDs 5..8)
    let ram_in_labels = ["Addr0", "Addr1", "CS", "WE", "D0", "D1", "D2", "D3"];
    let ram_in_y = [330.0, 370.0, 410.0, 450.0, 500.0, 540.0, 580.0, 620.0];
    for i in 0..8 {
        components.push(VisualComponent {
            id: 1 + i,
            comp_type: ComponentType::Input,
            pos: Vec2::new(150.0, ram_in_y[i]),
            width: 70.0,
            height: 30.0,
            label: ram_in_labels[i].to_string(),
            clock_period: None,
            bus_width: None,
            color: None,
        });
    }

    // RAM Subchip (ID 10) - Blueprint index 17
    components.push(VisualComponent {
        id: 10,
        comp_type: ComponentType::SubChip(17),
        pos: Vec2::new(340.0, 360.0),
        width: 180.0,
        height: 270.0,
        label: "RAM_4x4Bit".to_string(),
        clock_period: None,
        bus_width: None,
        color: None,
    });

    // Wire inputs to RAM subchip
    for i in 0..8 {
        connections.push(VisualConnection {
            src_comp_id: 1 + i,
            src_port: 0,
            tgt_comp_id: 10,
            tgt_port: i,
        });
    }

    // Outputs: Q0..Q3 (IDs 11..14)
    let ram_out_labels = ["Q0", "Q1", "Q2", "Q3"];
    let ram_out_y = [420.0, 470.0, 520.0, 570.0];
    for i in 0..4 {
        components.push(VisualComponent {
            id: 11 + i,
            comp_type: ComponentType::Output,
            pos: Vec2::new(600.0, ram_out_y[i]),
            width: 70.0,
            height: 30.0,
            label: ram_out_labels[i].to_string(),
            clock_period: None,
            bus_width: None,
            color: None,
        });
        connections.push(VisualConnection {
            src_comp_id: 10,
            src_port: i,
            tgt_comp_id: 11 + i,
            tgt_port: 0,
        });
    }

    // --- MODULE 2: Standalone 4-Bit Adder ---
    annotations.push(TextAnnotation {
        text: "=== MODULE 2: 4-BIT RIPPLE CARRY ADDER (Level 2) ===\nCalculates: [A3..A0] + [B3..B0] + Cin = [Cout, S3..S0]\nBuilt from Full Adders -> Half Adders -> XOR & AND -> Pure NANDs!".to_string(),
        pos: Vec2::new(800.0, 260.0),
    });

    let adder_in_labels = ["A0", "A1", "A2", "A3", "B0", "B1", "B2", "B3", "Cin"];
    let adder_in_y = [
        320.0, 355.0, 390.0, 425.0, 470.0, 505.0, 540.0, 575.0, 620.0,
    ];
    for i in 0..9 {
        components.push(VisualComponent {
            id: 20 + i,
            comp_type: ComponentType::Input,
            pos: Vec2::new(820.0, adder_in_y[i]),
            width: 60.0,
            height: 26.0,
            label: adder_in_labels[i].to_string(),
            clock_period: None,
            bus_width: None,
            color: None,
        });
    }

    // Adder Subchip (ID 30) - Blueprint index 8
    components.push(VisualComponent {
        id: 30,
        comp_type: ComponentType::SubChip(8),
        pos: Vec2::new(980.0, 370.0),
        width: 170.0,
        height: 250.0,
        label: "Adder4Bit".to_string(),
        clock_period: None,
        bus_width: None,
        color: None,
    });

    for i in 0..9 {
        connections.push(VisualConnection {
            src_comp_id: 20 + i,
            src_port: 0,
            tgt_comp_id: 30,
            tgt_port: i,
        });
    }

    // Adder Outputs (IDs 31..35)
    let adder_out_labels = ["S0", "S1", "S2", "S3", "Cout"];
    let adder_out_y = [390.0, 435.0, 480.0, 525.0, 570.0];
    for i in 0..5 {
        components.push(VisualComponent {
            id: 31 + i,
            comp_type: ComponentType::Output,
            pos: Vec2::new(1230.0, adder_out_y[i]),
            width: 70.0,
            height: 30.0,
            label: adder_out_labels[i].to_string(),
            clock_period: None,
            bus_width: None,
            color: None,
        });
        connections.push(VisualConnection {
            src_comp_id: 30,
            src_port: i,
            tgt_comp_id: 31 + i,
            tgt_port: 0,
        });
    }

    // --- MODULE 3: Integrated RAM + ALU Accumulator Unit ---
    annotations.push(TextAnnotation {
        text: "=== MODULE 3: INTEGRATED 4-BIT RAM + ALU ACCUMULATOR SYSTEM (Level 7) ===\nCombines the 4x4 RAM and the 4-bit ALU into an integrated CPU data-path:\nReads the selected word from RAM and adds it to the incoming Operand + Cin in real-time!\n- RAM Out [Q0..Q3]: Displays stored word at selected address.\n- ALU Result [Sum0..Sum3, Cout]: Displays (RAM_Word + Operand + Cin).".to_string(),
        pos: Vec2::new(100.0, 720.0),
    });

    let alu_in_labels = [
        "Addr0", "Addr1", "CS", "WE", "Op0", "Op1", "Op2", "Op3", "Cin",
    ];
    let alu_in_y = [
        780.0, 815.0, 850.0, 885.0, 930.0, 965.0, 1000.0, 1035.0, 1080.0,
    ];
    for i in 0..9 {
        components.push(VisualComponent {
            id: 40 + i,
            comp_type: ComponentType::Input,
            pos: Vec2::new(150.0, alu_in_y[i]),
            width: 70.0,
            height: 26.0,
            label: alu_in_labels[i].to_string(),
            clock_period: None,
            bus_width: None,
            color: None,
        });
    }

    // RAM4Bit_ALU_Unit Subchip (ID 50) - Blueprint index 18
    components.push(VisualComponent {
        id: 50,
        comp_type: ComponentType::SubChip(18),
        pos: Vec2::new(340.0, 810.0),
        width: 220.0,
        height: 290.0,
        label: "RAM4Bit_ALU_Unit".to_string(),
        clock_period: None,
        bus_width: None,
        color: None,
    });

    for i in 0..9 {
        connections.push(VisualConnection {
            src_comp_id: 40 + i,
            src_port: 0,
            tgt_comp_id: 50,
            tgt_port: i,
        });
    }

    // RAM outputs from ALU Unit (IDs 51..54)
    let alu_ram_out_labels = ["RAM_Q0", "RAM_Q1", "RAM_Q2", "RAM_Q3"];
    let alu_ram_out_y = [830.0, 870.0, 910.0, 950.0];
    for i in 0..4 {
        components.push(VisualComponent {
            id: 51 + i,
            comp_type: ComponentType::Output,
            pos: Vec2::new(650.0, alu_ram_out_y[i]),
            width: 80.0,
            height: 28.0,
            label: alu_ram_out_labels[i].to_string(),
            clock_period: None,
            bus_width: None,
            color: None,
        });
        connections.push(VisualConnection {
            src_comp_id: 50,
            src_port: i,
            tgt_comp_id: 51 + i,
            tgt_port: 0,
        });
    }

    // ALU Sum & Cout outputs from ALU Unit (IDs 55..59)
    let alu_sum_out_labels = ["Sum0", "Sum1", "Sum2", "Sum3", "Cout"];
    let alu_sum_out_y = [850.0, 890.0, 930.0, 970.0, 1010.0];
    for i in 0..5 {
        components.push(VisualComponent {
            id: 55 + i,
            comp_type: ComponentType::Output,
            pos: Vec2::new(820.0, alu_sum_out_y[i]),
            width: 80.0,
            height: 28.0,
            label: alu_sum_out_labels[i].to_string(),
            clock_period: None,
            bus_width: None,
            color: None,
        });
        connections.push(VisualConnection {
            src_comp_id: 50,
            src_port: 4 + i,
            tgt_comp_id: 55 + i,
            tgt_port: 0,
        });
    }

    ProjectFile {
        library,
        components,
        connections,
        next_component_id: 70,
        annotations,
        color_overrides: Default::default(),
        wire_nudges: Vec::new(),
    }
}

#[test]
fn test_save_and_verify_pure_nand_ram_project() {
    let project = create_pure_nand_ram_project();
    let json_data =
        serde_json::to_string_pretty(&project).expect("Failed to serialize ProjectFile");

    // Write both .logic and .json files
    std::fs::write("pure_nand_4bit_ram.logic", &json_data).expect("Failed to write .logic file");
    std::fs::write("pure_nand_4bit_ram.json", &json_data).expect("Failed to write .json file");

    // Test loading through Editor
    let mut editor = logic_simulator::editor::Editor::new();
    let loaded = editor.load_from_path("pure_nand_4bit_ram.logic");
    assert!(loaded, "Editor failed to load pure_nand_4bit_ram.logic");

    assert!(
        !editor.circuit.components.is_empty(),
        "Loaded components list is empty"
    );
    assert!(
        !editor.circuit.connections.is_empty(),
        "Loaded connections list is empty"
    );
    assert!(
        !editor.engine.simulator.nodes.is_empty(),
        "Engine simulator has no gates"
    );
    assert!(
        editor.engine.propagation_error.is_none(),
        "Engine encountered propagation error: {:?}",
        editor.engine.propagation_error
    );
}
