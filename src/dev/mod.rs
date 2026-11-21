//! The engine's stages as swappable parts and, on the desktop only, the
//! original engine under Wine as a reference, and where its installations
//! are.

#[cfg(not(target_arch = "wasm32"))]
pub mod compare;
#[cfg(not(target_arch = "wasm32"))]
pub mod engines;
#[cfg(not(target_arch = "wasm32"))]
pub mod oracle;
pub mod pipeline;
