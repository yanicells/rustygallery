#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Density {
    Small,
    Medium,
    Large,
}

impl Density {
    pub(crate) fn target(self) -> f32 {
        match self {
            Self::Small => 120.0,
            Self::Medium => 176.0,
            Self::Large => 248.0,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Small => "S",
            Self::Medium => "M",
            Self::Large => "L",
        }
    }

    pub(crate) fn as_pref(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }

    pub(crate) fn from_pref(value: &str) -> Self {
        match value {
            "small" => Self::Small,
            "large" => Self::Large,
            _ => Self::Medium,
        }
    }
}
