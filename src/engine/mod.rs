pub mod compiler;
pub mod profiler;
pub mod simulator;
pub mod sleep;
pub mod storage;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export everything for backward compatibility
pub use simulator::Simulator;
pub use sleep::SleepDomain;
pub use storage::{NO_SOURCE, SoAGateStorage};
pub use types::*;
