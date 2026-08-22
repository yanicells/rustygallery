pub(crate) const DEFAULT_IGNORES: &[&str] = &[
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "target",
    "dist",
    "build",
    "__pycache__",
    ".Trash",
    "Pods",
    "DerivedData",
    ".next",
    ".cache",
    "vendor",
];

pub(crate) fn default_ignore_list() -> Vec<String> {
    DEFAULT_IGNORES.iter().map(|s| (*s).to_string()).collect()
}

pub(crate) fn is_ignored(name: &str, names: &[String]) -> bool {
    let name = name.trim();
    !name.is_empty()
        && names
            .iter()
            .any(|d| name.eq_ignore_ascii_case(d.trim()) && !d.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_listed_names_only() {
        let names = default_ignore_list();
        assert!(is_ignored("node_modules", &names));
        assert!(is_ignored(".GIT", &names));
        assert!(!is_ignored("photos", &names));
        assert!(is_ignored("MyCache", &["mycache".into()]));
        assert!(!is_ignored("node_modules", &[]));
    }
}
