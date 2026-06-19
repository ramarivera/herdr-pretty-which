use ratatui::style::Color;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ThemeConfig {
    pub name: Option<String>,
    #[serde(default)]
    pub custom: ThemeCustom,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ThemeCustom {
    pub panel_bg: Option<String>,
    pub surface0: Option<String>,
    pub surface1: Option<String>,
    pub surface_dim: Option<String>,
    pub overlay0: Option<String>,
    pub overlay1: Option<String>,
    pub text: Option<String>,
    pub subtext0: Option<String>,
    pub accent: Option<String>,
    pub mauve: Option<String>,
    pub teal: Option<String>,
    pub peach: Option<String>,
    pub red: Option<String>,
    pub green: Option<String>,
    pub yellow: Option<String>,
    pub blue: Option<String>,
    pub magenta: Option<String>,
    pub cyan: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub panel: Color,
    pub panel_alt: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_2: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
}

impl Palette {
    pub fn from_theme(theme: &ThemeConfig) -> Self {
        let mut palette = match theme.name.as_deref().unwrap_or("terminal") {
            "catppuccin" | "catppuccin-mocha" => Self::catppuccin_mocha(),
            "catppuccin-latte" => Self::catppuccin_latte(),
            "catppuccin-frappe" => Self::catppuccin_frappe(),
            "catppuccin-macchiato" => Self::catppuccin_macchiato(),
            "tokyo-night" => Self::tokyo(),
            "dracula" => Self::dracula(),
            "nord" => Self::nord(),
            "gruvbox" => Self::gruvbox(),
            "one-dark" => Self::one_dark(),
            "solarized" => Self::solarized(),
            "kanagawa" => Self::kanagawa(),
            "rose-pine" => Self::rose_pine(),
            "vesper" => Self::vesper(),
            "terminal" => Self::terminal(),
            _ => Self::terminal(),
        };

        if let Some(panel) = theme.custom.panel_bg.as_deref().and_then(parse_color) {
            palette.panel = panel;
            if is_light(panel) && matches!(theme.name.as_deref(), Some("catppuccin")) {
                palette = Self::catppuccin_latte();
                palette.panel = panel;
            }
        }
        if let Some(surface) = theme.custom.surface0.as_deref().and_then(parse_color) {
            palette.panel_alt = surface;
        }
        if let Some(text) = theme.custom.text.as_deref().and_then(parse_color) {
            palette.text = text;
        }
        if let Some(muted) = theme.custom.subtext0.as_deref().and_then(parse_color) {
            palette.muted = muted;
        } else if let Some(muted) = theme.custom.overlay0.as_deref().and_then(parse_color) {
            palette.muted = muted;
        }
        if let Some(accent) = theme
            .custom
            .accent
            .as_deref()
            .or(theme.custom.mauve.as_deref())
            .or(theme.custom.blue.as_deref())
            .and_then(parse_color)
        {
            palette.accent = accent;
        }
        if let Some(accent_2) = theme
            .custom
            .teal
            .as_deref()
            .or(theme.custom.cyan.as_deref())
            .and_then(parse_color)
        {
            palette.accent_2 = accent_2;
        }
        if let Some(red) = theme.custom.red.as_deref().and_then(parse_color) {
            palette.danger = red;
        }
        if let Some(green) = theme.custom.green.as_deref().and_then(parse_color) {
            palette.success = green;
        }
        if let Some(yellow) = theme.custom.yellow.as_deref().and_then(parse_color) {
            palette.warning = yellow;
        }
        palette
    }

    pub fn selected_text_color(self) -> Color {
        Self::selected_text_color_for(self.bg, self.accent)
    }

    pub fn selected_text_color_for(intended_text: Color, selected_background: Color) -> Color {
        const MIN_CONTRAST: f64 = 4.5;
        if contrast_ratio(intended_text, selected_background) >= MIN_CONTRAST {
            return intended_text;
        }

        let Some((text_r, text_g, text_b)) = color_rgb(intended_text) else {
            return if is_light(selected_background) {
                Color::Black
            } else {
                Color::White
            };
        };
        let endpoints = [Color::Black, Color::White];
        endpoints
            .into_iter()
            .filter_map(|endpoint| {
                let (end_r, end_g, end_b) = color_rgb(endpoint)?;
                if contrast_ratio(endpoint, selected_background) < MIN_CONTRAST {
                    return None;
                }
                let mut low = 0.0;
                let mut high = 1.0;
                for _ in 0..16 {
                    let mid = (low + high) / 2.0;
                    let candidate = Color::Rgb(
                        lerp_u8(text_r, end_r, mid),
                        lerp_u8(text_g, end_g, mid),
                        lerp_u8(text_b, end_b, mid),
                    );
                    if contrast_ratio(candidate, selected_background) >= MIN_CONTRAST {
                        high = mid;
                    } else {
                        low = mid;
                    }
                }
                let candidate = Color::Rgb(
                    lerp_u8(text_r, end_r, high),
                    lerp_u8(text_g, end_g, high),
                    lerp_u8(text_b, end_b, high),
                );
                Some((high, candidate))
            })
            .min_by(|left, right| left.0.total_cmp(&right.0))
            .map(|(_, color)| color)
            .unwrap_or_else(|| {
                if contrast_ratio(Color::Black, selected_background)
                    >= contrast_ratio(Color::White, selected_background)
                {
                    Color::Black
                } else {
                    Color::White
                }
            })
    }

    fn terminal() -> Self {
        Self {
            bg: Color::Reset,
            panel: Color::Black,
            panel_alt: Color::DarkGray,
            text: Color::White,
            muted: Color::Gray,
            accent: Color::Cyan,
            accent_2: Color::Blue,
            success: Color::Green,
            warning: Color::Yellow,
            danger: Color::Red,
        }
    }

    fn catppuccin_mocha() -> Self {
        Self {
            bg: Color::Rgb(30, 30, 46),
            panel: Color::Rgb(24, 24, 37),
            panel_alt: Color::Rgb(49, 50, 68),
            text: Color::Rgb(205, 214, 244),
            muted: Color::Rgb(166, 173, 200),
            accent: Color::Rgb(137, 180, 250),
            accent_2: Color::Rgb(148, 226, 213),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            danger: Color::Rgb(243, 139, 168),
        }
    }

    fn catppuccin_latte() -> Self {
        Self {
            bg: Color::Rgb(239, 241, 245),
            panel: Color::Rgb(239, 241, 245),
            panel_alt: Color::Rgb(204, 208, 218),
            text: Color::Rgb(76, 79, 105),
            muted: Color::Rgb(108, 111, 133),
            accent: Color::Rgb(136, 57, 239),
            accent_2: Color::Rgb(23, 146, 153),
            success: Color::Rgb(64, 160, 43),
            warning: Color::Rgb(223, 142, 29),
            danger: Color::Rgb(210, 15, 57),
        }
    }

    fn catppuccin_frappe() -> Self {
        Self {
            bg: Color::Rgb(48, 52, 70),
            panel: Color::Rgb(41, 44, 60),
            panel_alt: Color::Rgb(65, 69, 89),
            text: Color::Rgb(198, 208, 245),
            muted: Color::Rgb(165, 173, 206),
            accent: Color::Rgb(202, 158, 230),
            accent_2: Color::Rgb(129, 200, 190),
            success: Color::Rgb(166, 209, 137),
            warning: Color::Rgb(229, 200, 144),
            danger: Color::Rgb(231, 130, 132),
        }
    }

    fn catppuccin_macchiato() -> Self {
        Self {
            bg: Color::Rgb(36, 39, 58),
            panel: Color::Rgb(30, 32, 48),
            panel_alt: Color::Rgb(54, 58, 79),
            text: Color::Rgb(202, 211, 245),
            muted: Color::Rgb(165, 173, 203),
            accent: Color::Rgb(198, 160, 246),
            accent_2: Color::Rgb(139, 213, 202),
            success: Color::Rgb(166, 218, 149),
            warning: Color::Rgb(238, 212, 159),
            danger: Color::Rgb(237, 135, 150),
        }
    }

    fn tokyo() -> Self {
        Self {
            bg: Color::Rgb(26, 27, 38),
            panel: Color::Rgb(22, 22, 30),
            panel_alt: Color::Rgb(41, 46, 66),
            text: Color::Rgb(192, 202, 245),
            muted: Color::Rgb(122, 162, 247),
            accent: Color::Rgb(125, 207, 255),
            accent_2: Color::Rgb(187, 154, 247),
            success: Color::Rgb(158, 206, 106),
            warning: Color::Rgb(224, 175, 104),
            danger: Color::Rgb(247, 118, 142),
        }
    }

    fn dracula() -> Self {
        Self {
            bg: Color::Rgb(40, 42, 54),
            panel: Color::Rgb(33, 34, 44),
            panel_alt: Color::Rgb(68, 71, 90),
            text: Color::Rgb(248, 248, 242),
            muted: Color::Rgb(98, 114, 164),
            accent: Color::Rgb(189, 147, 249),
            accent_2: Color::Rgb(139, 233, 253),
            success: Color::Rgb(80, 250, 123),
            warning: Color::Rgb(241, 250, 140),
            danger: Color::Rgb(255, 85, 85),
        }
    }

    fn nord() -> Self {
        Self {
            bg: Color::Rgb(46, 52, 64),
            panel: Color::Rgb(36, 41, 51),
            panel_alt: Color::Rgb(59, 66, 82),
            text: Color::Rgb(236, 239, 244),
            muted: Color::Rgb(129, 161, 193),
            accent: Color::Rgb(136, 192, 208),
            accent_2: Color::Rgb(180, 142, 173),
            success: Color::Rgb(163, 190, 140),
            warning: Color::Rgb(235, 203, 139),
            danger: Color::Rgb(191, 97, 106),
        }
    }

    fn gruvbox() -> Self {
        Self {
            bg: Color::Rgb(40, 40, 40),
            panel: Color::Rgb(29, 32, 33),
            panel_alt: Color::Rgb(60, 56, 54),
            text: Color::Rgb(235, 219, 178),
            muted: Color::Rgb(168, 153, 132),
            accent: Color::Rgb(131, 165, 152),
            accent_2: Color::Rgb(142, 192, 124),
            success: Color::Rgb(184, 187, 38),
            warning: Color::Rgb(250, 189, 47),
            danger: Color::Rgb(251, 73, 52),
        }
    }

    fn one_dark() -> Self {
        Self {
            bg: Color::Rgb(40, 44, 52),
            panel: Color::Rgb(33, 37, 43),
            panel_alt: Color::Rgb(62, 68, 81),
            text: Color::Rgb(171, 178, 191),
            muted: Color::Rgb(130, 137, 151),
            accent: Color::Rgb(198, 120, 221),
            accent_2: Color::Rgb(86, 182, 194),
            success: Color::Rgb(152, 195, 121),
            warning: Color::Rgb(229, 192, 123),
            danger: Color::Rgb(224, 108, 117),
        }
    }

    fn solarized() -> Self {
        Self {
            bg: Color::Rgb(0, 43, 54),
            panel: Color::Rgb(7, 54, 66),
            panel_alt: Color::Rgb(88, 110, 117),
            text: Color::Rgb(238, 232, 213),
            muted: Color::Rgb(147, 161, 161),
            accent: Color::Rgb(38, 139, 210),
            accent_2: Color::Rgb(42, 161, 152),
            success: Color::Rgb(133, 153, 0),
            warning: Color::Rgb(181, 137, 0),
            danger: Color::Rgb(220, 50, 47),
        }
    }

    fn kanagawa() -> Self {
        Self {
            bg: Color::Rgb(31, 31, 40),
            panel: Color::Rgb(22, 22, 29),
            panel_alt: Color::Rgb(54, 54, 69),
            text: Color::Rgb(220, 215, 186),
            muted: Color::Rgb(114, 123, 137),
            accent: Color::Rgb(149, 127, 184),
            accent_2: Color::Rgb(126, 156, 216),
            success: Color::Rgb(152, 187, 108),
            warning: Color::Rgb(230, 195, 132),
            danger: Color::Rgb(196, 91, 102),
        }
    }

    fn rose_pine() -> Self {
        Self {
            bg: Color::Rgb(25, 23, 36),
            panel: Color::Rgb(31, 29, 46),
            panel_alt: Color::Rgb(49, 45, 75),
            text: Color::Rgb(224, 222, 244),
            muted: Color::Rgb(144, 140, 170),
            accent: Color::Rgb(196, 167, 231),
            accent_2: Color::Rgb(156, 207, 216),
            success: Color::Rgb(49, 116, 143),
            warning: Color::Rgb(246, 193, 119),
            danger: Color::Rgb(235, 111, 146),
        }
    }

    fn vesper() -> Self {
        Self {
            bg: Color::Rgb(16, 16, 18),
            panel: Color::Rgb(22, 22, 25),
            panel_alt: Color::Rgb(48, 48, 52),
            text: Color::Rgb(218, 218, 218),
            muted: Color::Rgb(135, 135, 140),
            accent: Color::Rgb(255, 128, 102),
            accent_2: Color::Rgb(153, 209, 219),
            success: Color::Rgb(153, 199, 148),
            warning: Color::Rgb(255, 204, 102),
            danger: Color::Rgb(237, 135, 150),
        }
    }
}

fn is_light(color: Color) -> bool {
    color_rgb(color)
        .map(|(red, green, blue)| {
            let luminance = (0.2126 * f64::from(red))
                + (0.7152 * f64::from(green))
                + (0.0722 * f64::from(blue));
            luminance > 155.0
        })
        .unwrap_or(false)
}

fn contrast_ratio(foreground: Color, background: Color) -> f64 {
    let Some((fg_red, fg_green, fg_blue)) = color_rgb(foreground) else {
        return 21.0;
    };
    let Some((bg_red, bg_green, bg_blue)) = color_rgb(background) else {
        return 21.0;
    };
    let fg_luminance = relative_luminance(fg_red, fg_green, fg_blue);
    let bg_luminance = relative_luminance(bg_red, bg_green, bg_blue);
    let lighter = fg_luminance.max(bg_luminance);
    let darker = fg_luminance.min(bg_luminance);
    (lighter + 0.05) / (darker + 0.05)
}

fn relative_luminance(red: u8, green: u8, blue: u8) -> f64 {
    fn channel(value: u8) -> f64 {
        let normalized = f64::from(value) / 255.0;
        if normalized <= 0.03928 {
            normalized / 12.92
        } else {
            ((normalized + 0.055) / 1.055).powf(2.4)
        }
    }
    (0.2126 * channel(red)) + (0.7152 * channel(green)) + (0.0722 * channel(blue))
}

fn color_rgb(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Black => Some((0, 0, 0)),
        Color::Red => Some((255, 0, 0)),
        Color::Green => Some((0, 128, 0)),
        Color::Yellow => Some((255, 255, 0)),
        Color::Blue => Some((0, 0, 255)),
        Color::Magenta => Some((255, 0, 255)),
        Color::Cyan => Some((0, 255, 255)),
        Color::Gray => Some((128, 128, 128)),
        Color::DarkGray => Some((64, 64, 64)),
        Color::LightRed => Some((255, 85, 85)),
        Color::LightGreen => Some((85, 255, 85)),
        Color::LightYellow => Some((255, 255, 85)),
        Color::LightBlue => Some((85, 85, 255)),
        Color::LightMagenta => Some((255, 85, 255)),
        Color::LightCyan => Some((85, 255, 255)),
        Color::White => Some((255, 255, 255)),
        Color::Rgb(red, green, blue) => Some((red, green, blue)),
        _ => None,
    }
}

fn lerp_u8(from: u8, to: u8, t: f64) -> u8 {
    (f64::from(from) + ((f64::from(to) - f64::from(from)) * t))
        .round()
        .clamp(0.0, 255.0) as u8
}

fn parse_color(value: &str) -> Option<Color> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("reset") {
        return Some(Color::Reset);
    }
    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() == 6 {
            let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::Rgb(red, green, blue));
        }
    }
    match value.to_ascii_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_color() {
        assert_eq!(parse_color("#89b4fa"), Some(Color::Rgb(137, 180, 250)));
    }

    #[test]
    fn custom_accent_overrides_theme() {
        let theme = ThemeConfig {
            name: Some("catppuccin".to_string()),
            custom: ThemeCustom {
                accent: Some("#ffffff".to_string()),
                ..Default::default()
            },
        };
        assert_eq!(
            Palette::from_theme(&theme).accent,
            Color::Rgb(255, 255, 255)
        );
    }

    #[test]
    fn selected_text_color_keeps_target_when_contrast_is_already_good() {
        assert_eq!(
            Palette::selected_text_color_for(Color::Rgb(0, 0, 0), Color::Rgb(255, 255, 255)),
            Color::Rgb(0, 0, 0)
        );
    }

    #[test]
    fn selected_text_color_moves_toward_legibility_when_target_contrast_is_bad() {
        let adjusted =
            Palette::selected_text_color_for(Color::Rgb(150, 80, 230), Color::Rgb(136, 57, 239));

        assert_ne!(adjusted, Color::Rgb(150, 80, 230));
        assert!(contrast_ratio(adjusted, Color::Rgb(136, 57, 239)) >= 4.5);
    }

    #[test]
    fn live_latte_custom_tokens_override_catppuccin_base() {
        let theme = ThemeConfig {
            name: Some("catppuccin".to_string()),
            custom: ThemeCustom {
                panel_bg: Some("#eff1f5".to_string()),
                surface0: Some("#ccd0da".to_string()),
                text: Some("#4c4f69".to_string()),
                subtext0: Some("#6c6f85".to_string()),
                mauve: Some("#8839ef".to_string()),
                teal: Some("#179299".to_string()),
                green: Some("#40a02b".to_string()),
                yellow: Some("#df8e1d".to_string()),
                red: Some("#d20f39".to_string()),
                ..Default::default()
            },
        };
        let palette = Palette::from_theme(&theme);
        assert_eq!(palette.panel, Color::Rgb(239, 241, 245));
        assert_eq!(palette.panel_alt, Color::Rgb(204, 208, 218));
        assert_eq!(palette.text, Color::Rgb(76, 79, 105));
        assert_eq!(palette.muted, Color::Rgb(108, 111, 133));
        assert_eq!(palette.accent, Color::Rgb(136, 57, 239));
        assert_eq!(palette.accent_2, Color::Rgb(23, 146, 153));
    }
}
