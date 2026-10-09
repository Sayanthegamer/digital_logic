pub mod compiler;
pub mod profiler;
pub mod simulator;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export everything for backward compatibility
pub use simulator::Simulator;
pub use types::*;
