pub mod math;
pub mod fractal;
pub mod memory;
pub mod mind;
pub mod llm;
pub mod visualizer;

pub use math::{C, Param, param_to_c};
pub use fractal::{
    Fingerprint, MandelbrotCoord, julia_escapes, mandelbrot_escapes,
    julia_fingerprint_from_mandelbrot, hash_to_mandelbrot_coord,
    contextual_mandelbrot_coord, mandelbrot_stability, hamming_distance
};
pub use memory::{FractalSymbol, ConceptTrajectory, AssociativeField};
pub use mind::FractalMind;
pub use llm::LLMBridge;
pub use visualizer::spawn_visualizer;