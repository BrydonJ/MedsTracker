//! Medicine schedule, weekly packing plan and stock maths. No UI dependencies.

mod model;
mod plan;

pub use model::*;
pub use plan::*;

#[cfg(test)]
mod tests;
