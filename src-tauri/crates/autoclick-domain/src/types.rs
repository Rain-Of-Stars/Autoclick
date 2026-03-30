use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Roi {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Roi {
    pub fn is_full_frame(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorRef {
    pub id: String,
    pub name: String,
    pub is_primary: bool,
}

impl Default for MonitorRef {
    fn default() -> Self {
        Self {
            id: "primary".to_string(),
            name: "主显示器".to_string(),
            is_primary: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum RuntimeStatus {
    #[default]
    Idle,
    Starting,
    Running,
    CoolingDown,
    Recovering,
    Stopping,
    Faulted,
}
