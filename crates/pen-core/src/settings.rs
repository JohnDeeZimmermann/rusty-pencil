use crate::Srgba;
use serde::{Deserialize, Serialize};

/// Persisted editor preferences. Grid spacing is 20 world points.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(default)]
pub struct Settings {
    pub dark: bool,
    pub grid: bool,
    pub tool_lock: bool,
    pub canvas: Srgba,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            dark: false,
            grid: false,
            tool_lock: false,
            canvas: Srgba::rgb(0xf8f9f7),
        }
    }
}
