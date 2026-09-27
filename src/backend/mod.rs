//! Integração com backends de temperatura de cor no desktop.

mod conflicts;
mod sunsetr;

pub use conflicts::{ConflictReport, detect_conflicts};
pub use sunsetr::{
    BackendKind, Period, RuntimeStatus, SunsetrClient, SunsetrConfig, TransitionMode, UiPreset,
};
