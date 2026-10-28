use super::{contrast_ratio, Palette};
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

/// Resolve styles after widgets have inherited their actual backgrounds.
/// Terminal-backed colors must remain terminal-backed: Reset has no RGB value
/// and a dark preset cannot describe a terminal that switches appearance.
pub(crate) fn ensure_readable(buffer: &mut Buffer, palette: Palette) {
    let terminal_backed = palette.text == Color::Reset || palette.bg == Color::Reset;
    for cell in &mut buffer.content {
        if terminal_backed {
            if cell.bg == palette.accent && palette.accent != Color::Reset {
                cell.modifier.insert(Modifier::REVERSED);
            }
            cell.fg = Color::Reset;
            cell.bg = Color::Reset;
        } else if contrast_ratio(cell.fg, cell.bg) < 4.5 {
            cell.fg = Palette::selected_text_color_for(cell.fg, cell.bg);
        }
    }
}
