use crate::media::Entry;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SortKey {
    Name,
    Modified,
    Size,
    Type,
}

impl SortKey {
    pub(crate) fn as_pref(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Modified => "modified",
            Self::Size => "size",
            Self::Type => "type",
        }
    }

    pub(crate) fn from_pref(value: &str) -> Self {
        match value {
            "modified" => Self::Modified,
            "size" => Self::Size,
            "type" => Self::Type,
            _ => Self::Name,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Modified => "Date",
            Self::Size => "Size",
            Self::Type => "Type",
        }
    }

    pub(crate) fn next(self) -> Self {
        match self {
            Self::Name => Self::Modified,
            Self::Modified => Self::Size,
            Self::Size => Self::Type,
            Self::Type => Self::Name,
        }
    }
}

pub(crate) fn sort_entries(entries: &mut [Entry], key: SortKey, desc: bool) {
    entries.sort_by(|a, b| {
        let a_folder = matches!(a, Entry::Folder(_));
        let b_folder = matches!(b, Entry::Folder(_));
        if a_folder != b_folder {
            return b_folder.cmp(&a_folder);
        }
        let ord = match key {
            SortKey::Name => a.name().to_lowercase().cmp(&b.name().to_lowercase()),
            SortKey::Modified => a.modified().cmp(&b.modified()),
            SortKey::Size => a.size().cmp(&b.size()),
            SortKey::Type => a
                .type_key()
                .cmp(&b.type_key())
                .then_with(|| a.name().to_lowercase().cmp(&b.name().to_lowercase())),
        };
        if desc {
            ord.reverse()
        } else {
            ord
        }
    });
}
