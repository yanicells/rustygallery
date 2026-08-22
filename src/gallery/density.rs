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
}
