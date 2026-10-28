use super::{contrast_ratio, Palette};
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

/// Resolve styles after widgets have inherited their actual backgrounds.
/// Terminal-backed colors use the terminal's live ANSI palette, rather than
/// stale RGB values from a preset that cannot follow appearance changes.
pub(crate) fn ensure_readable(buffer: &mut Buffer, palette: Palette) {
    let terminal_backed = palette.text == Color::Reset || palette.bg == Color::Reset;
    for cell in &mut buffer.content {
        if terminal_backed {
            if cell.bg == palette.accent && palette.accent != Color::Reset {
                cell.modifier.insert(Modifier::REVERSED);
                cell.fg = Color::Reset;
            } else if cell.fg == palette.accent || cell.fg == palette.success {
                cell.fg = Color::Blue;
            } else if cell.fg == palette.accent_2 || cell.fg == palette.warning {
                cell.fg = Color::Magenta;
            } else if cell.fg == palette.danger {
                cell.fg = Color::Red;
            } else {
                if cell.fg == palette.panel_alt {
                    cell.modifier.insert(Modifier::DIM);
                }
                cell.fg = Color::Reset;
            }
            cell.bg = Color::Reset;
        } else if contrast_ratio(cell.fg, cell.bg) < 4.5 {
            cell.fg = Palette::selected_text_color_for(cell.fg, cell.bg);
        }
    }
}
