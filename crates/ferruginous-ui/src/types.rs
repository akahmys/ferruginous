use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ToolMode {
    Select,
    Snap,
    Measure,
}

impl ToolMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Select => "選択",
            Self::Snap => "スナップ",
            Self::Measure => "計測",
        }
    }
}
