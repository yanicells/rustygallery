use std::cell::Cell;

use gpui::WindowAppearance;

/// Named colors for chrome. Surfaces read `Theme::current()` after render sets it.
#[derive(Clone, Copy)]
pub struct Theme {
    pub bg: u32,
    pub surface: u32,
    pub surface_hover: u32,
    pub tile: u32,
    pub tile_folder: u32,
    pub tile_media: u32,
    pub border: u32,
    pub text: u32,
    pub text_muted: u32,
    pub text_dim: u32,
    pub text_faint: u32,
    pub text_hint: u32,
    pub accent: u32,
    pub accent_soft: u32,
    pub inactive: u32,
    pub name_idle: u32,
    pub btn: u32,
    pub btn_active: u32,
    pub btn_hover: u32,
    pub btn_text: u32,
    pub row_active: u32,
    pub prominent: u32,
    pub prominent_text: u32,
    pub prominent_hover: u32,
    pub lightbox: u32,
    pub on_accent: u32,
}

thread_local! {
    static CURRENT: Cell<Theme> = const { Cell::new(Theme::DARK) };
}

impl Theme {
    pub const DARK: Self = Self {
        bg: 0x101010,
        surface: 0x141414,
        surface_hover: 0x222222,
        tile: 0x222222,
        tile_folder: 0x1c1c1c,
        tile_media: 0x1a1a1a,
        border: 0x242424,
        text: 0xe8e8e8,
        text_muted: 0x888888,
        text_dim: 0x777777,
        text_faint: 0x666666,
        text_hint: 0x555555,
        accent: 0xf0f0f0,
        accent_soft: 0xd0d0d0,
        inactive: 0xa8a8a8,
        name_idle: 0x8a8a8a,
        btn: 0x242424,
        btn_active: 0x3a3a3a,
        btn_hover: 0x303030,
        btn_text: 0xc8c8c8,
        row_active: 0x2e2e2e,
        prominent: 0xe8e8e8,
        prominent_text: 0x111111,
        prominent_hover: 0xffffff,
        lightbox: 0x0a0a0a,
        on_accent: 0xffffff,
    };

    pub const LIGHT: Self = Self {
        bg: 0xf4f4f2,
        surface: 0xecece8,
        surface_hover: 0xe0e0dc,
        tile: 0xe4e4e0,
        tile_folder: 0xdddcd6,
        tile_media: 0xe8e8e4,
        border: 0xd0d0ca,
        text: 0x1a1a18,
        text_muted: 0x5c5c56,
        text_dim: 0x6a6a64,
        text_faint: 0x8a8a84,
        text_hint: 0x9a9a94,
        accent: 0x1a1a18,
        accent_soft: 0x3a3a36,
        inactive: 0x6a6a64,
        name_idle: 0x5a5a54,
        btn: 0xe0e0da,
        btn_active: 0xd0d0ca,
        btn_hover: 0xd8d8d2,
        btn_text: 0x2a2a26,
        row_active: 0xd4d4ce,
        prominent: 0x1a1a18,
        prominent_text: 0xf4f4f2,
        prominent_hover: 0x000000,
        lightbox: 0xf7f7f4,
        on_accent: 0x1a1a18,
    };

    pub fn resolve(pref: &str, appearance: WindowAppearance) -> Self {
        match pref {
            "light" => Self::LIGHT,
            "system"
                if matches!(
                    appearance,
                    WindowAppearance::Light | WindowAppearance::VibrantLight
                ) =>
            {
                Self::LIGHT
            }
            _ => Self::DARK,
        }
    }

    pub fn set_current(theme: Self) {
        CURRENT.set(theme);
    }

    pub fn current() -> Self {
        CURRENT.get()
    }
}

#[cfg(test)]
mod tests {
    use super::Theme;
    use gpui::WindowAppearance;

    #[test]
    fn resolve_follows_pref_then_system() {
        assert_eq!(
            Theme::resolve("light", WindowAppearance::Dark).bg,
            Theme::LIGHT.bg
        );
        assert_eq!(
            Theme::resolve("system", WindowAppearance::Light).bg,
            Theme::LIGHT.bg
        );
        assert_eq!(
            Theme::resolve("system", WindowAppearance::Dark).bg,
            Theme::DARK.bg
        );
        assert_eq!(
            Theme::resolve("dark", WindowAppearance::Light).bg,
            Theme::DARK.bg
        );
    }
}
