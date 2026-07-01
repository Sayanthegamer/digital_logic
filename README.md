# Digital Logic Simulator

[![Publish Release](https://github.com/Sayanthegamer/digital_logic/actions/workflows/release.yml/badge.svg)](https://github.com/Sayanthegamer/digital_logic/actions/workflows/release.yml)
[![Build Windows EXE](https://github.com/Sayanthegamer/digital_logic/actions/workflows/windows.yml/badge.svg)](https://github.com/Sayanthegamer/digital_logic/actions/workflows/windows.yml)
[![Build Android APK](https://github.com/Sayanthegamer/digital_logic/actions/workflows/android.yml/badge.svg)](https://github.com/Sayanthegamer/digital_logic/actions/workflows/android.yml)
[![VirusTotal Scan](https://img.shields.io/badge/VirusTotal-Scanned-brightgreen?logo=virustotal)](https://www.virustotal.com/)
[![Rust](https://img.shields.io/badge/rust-edition%202024-blue.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Android-lightgrey.svg)]()
[![Made with Macroquad](https://img.shields.io/badge/Made%20with-Macroquad-green.svg)](https://macroquad.rs)

A high-performance, Rust-based Digital Logic Simulator.

This project was built with a specific goal in mind: **to rival and out-perform Sebastian Lague's "Digital Logic Simulator" by a landslide in pure processing power, enabling the creation of fully functional 8-bit and 16-bit CPUs without lag.**

## Why this exists
Many visual logic simulators (including those built in Unity/C#) suffer from Object-Oriented Programming (OOP) bottlenecks. As you abstract chips into nested sub-chips, the simulator has to perform costly tree-traversals or virtual method calls at runtime. Eventually, attempting to run a full CPU in real-time causes massive framerate drops.

**This simulator solves that problem.**

By utilizing a Data-Oriented Design (struct-of-arrays) and a completely flat compilation step, nested chips have **zero runtime overhead**. When you build an ALU from Adders, and a CPU from ALUs, the compiler unwraps the entire hierarchy down to raw, contiguous NAND arrays. The simulation engine only ever processes flat arrays of boolean states, making it incredibly cache-friendly and extremely fast.

## Architecture Overview

```mermaid
flowchart TB
    %% Styling
    classDef frontend fill:#f9f9f9,stroke:#333,stroke-width:2px;
    classDef backend fill:#eef2f5,stroke:#2a5078,stroke-width:2px;
    classDef core fill:#d4e1f9,stroke:#1a365d,stroke-width:2px,font-weight:bold;

    subgraph Frontend["Frontend / IDE (src/editor)"]
        direction TB
        Input[Input Handler<br/><code>input.rs</code>]
        Canvas[Visual Canvas<br/><code>canvas.rs</code>]
        Draw[Rendering Engine<br/><code>drawing.rs & drawing_wires.rs</code>]
        UI[Immediate Mode UI<br/><code>ui_catalog.rs & ui_properties.rs</code>]
        Inspect[Inspection Logic<br/><code>inspection_ui.rs & inspection_logic.rs</code>]
        History[Undo/Redo Stack<br/><code>history.rs</code>]
    end

    subgraph Backend["Backend / Simulation Engine (src/engine)"]
        direction TB
        Compiler[Compiler<br/><code>compiler.rs</code>]
        Simulator[Event-Driven Simulator<br/><code>simulator.rs</code>]
        Types[Data Types & Blueprint<br/><code>types.rs</code>]
        SaveLoad[Serialization<br/><code>save_load.rs</code>]
    end

    %% Flow of data
    User((User)) -->|Mouse / Keyboard| Input
    Input -->|Modifies| Canvas
    Input -->|Records Action| History
    History -->|Restores State| Canvas
    Canvas -->|Builds| Types
    UI -->|Changes Tools / Props| Input
    SaveLoad -.->|Persists to Disk| Types

    %% Compilation Flow
    Canvas ==>|Triggers Compilation| Compiler
    Types -->|Reads Blueprints| Compiler
    Compiler ==>|Flattens Hierarchy & Resolves Bus Nets| Simulator

    %% Simulation & Render Flow
    Simulator -->|4-State Logic Updates| Draw
    Canvas -->|Viewport Data| Draw
    Draw -->|Renders 2D Graphics| User
    
    %% Inspection Flow
    Inspect -.->|Deep Tracing| Types
    Inspect -.->|Live Probes| Simulator
    Inspect -->|Overlays Data| Draw

    class Frontend frontend;
    class Backend backend;
    class Simulator core;
    class Compiler core;
```

## Features
- **Event-Driven Engine**: Gates only calculate when their inputs change, vastly reducing CPU load compared to tick-based simulators.
- **Flat Sub-Chip Compilation**: Build complex nested logic without sacrificing a single frame of performance.
- **Multi-Domain Clocks**: Native support for clocks with localized periods running synchronously.
- **Oscillation Detection**: Prevents the application from freezing when infinite zero-delay feedback loops are accidentally created.
- **Clean UI**: Built with Macroquad (for high-speed 2D canvas rendering) and egui (for the immediate-mode interface), fully modularized for easy expansion and testing.
- **Multi-Platform & Mobile Ready**: Compiles natively to Windows, Android (featuring touch screen pan/zoom controls).


## Documentation
- [Architecture](ARCHITECTURE.md)
- [Design Philosophy](DESIGN.md)
- [System Specifications](SPEC.md)
- [System Requirements](SYSTEM.md)
- [Build & Deployment](DEPLOYMENT.md)

## Quick Start
Ensure you have Rust installed (and Linux dependencies if applicable, see [SYSTEM.md](SYSTEM.md)).

```bash
# Run for testing
cargo run

# Run for maximum performance (recommended when building CPUs)
cargo run --release
```
