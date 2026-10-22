//! KEYS view: which-key style category columns with key badges.
//!
//! Layout mirrors the README cover: a title bar, a filter row with a legend, a
//! row of category columns, then a details line and a controls footer. Every
//! badge comes from the loaded bindings; nothing here invents keys or actions.
//! Cross-reference: `crate::app::App::key_groups` and `crate::render`.

use crate::app::{App, KeyGroup};
use crate::model::{Binding, BindingSource, BindingStatus, Category};
use crate::theme::Palette;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Narrowest column before the view drops to fewer columns.
const MIN_COLUMN_WIDTH: u16 = 34;
/// Widest key badge before it is truncated with an ellipsis.
const MAX_BADGE_WIDTH: usize = 14;
/// Below this width the legend and the config/theme summary are hidden.
const COMPACT_WIDTH: u16 = 72;

/// Which layer a key lives on, used to color its badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyLayer {
    /// `prefix+…` chords, shown as the key pressed after the prefix.
    Prefix,
    /// Chords that fire without the prefix (`alt+…`, `ctrl+alt+…`).
    Direct,
    /// No key is assigned.
    Unset,
}

/// Badge text and layer for a binding's primary key.
pub fn key_badge(binding: &Binding) -> (String, KeyLayer) {
    match binding.keys.first() {
        None => ("—".to_string(), KeyLayer::Unset),
        Some(key) => match key.strip_prefix("prefix+") {
            Some(rest) if !rest.is_empty() => (rest.to_string(), KeyLayer::Prefix),
            _ => (key.clone(), KeyLayer::Direct),
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    Header {
        category: Category,
        count: usize,
        continued: bool,
    },
    Item {
        index: usize,
        last_in_group: bool,
    },
    Blank,
}

/// Pack groups into fixed-height columns. Headers never sit alone at the bottom
/// of a column, and a group that spills over repeats its header.
fn pack_columns(groups: &[KeyGroup], height: usize) -> Vec<Vec<Entry>> {
    let height = height.max(2);
    let mut columns: Vec<Vec<Entry>> = vec![Vec::new()];
    let mut index = 0usize;
    for group in groups {
        let count = group.bindings.len();
        {
            let column = columns.last_mut().expect("columns is never empty");
            if !column.is_empty() && column.len() + 3 > height {
                columns.push(Vec::new());
            } else if !column.is_empty() {
                column.push(Entry::Blank);
            }
        }
        columns.last_mut().unwrap().push(Entry::Header {
            category: group.category,
            count,
            continued: false,
        });
        for position in 0..count {
            if columns.last().unwrap().len() >= height {
                columns.push(vec![Entry::Header {
                    category: group.category,
                    count,
                    continued: true,
                }]);
            }
            columns.last_mut().unwrap().push(Entry::Item {
                index,
                last_in_group: position + 1 == count,
            });
            index += 1;
        }
    }
    if columns.len() > 1 && columns.last().is_some_and(Vec::is_empty) {
        columns.pop();
    }
    columns
}

fn column_of(columns: &[Vec<Entry>], selected: usize) -> usize {
    columns
        .iter()
        .position(|column| {
            column
                .iter()
                .any(|entry| matches!(entry, Entry::Item { index, .. } if *index == selected))
        })
        .unwrap_or(0)
}

const fn category_icon(category: Category) -> &'static str {
    match category {
        Category::Core => "◆",
        Category::Workspaces => "▦",
        Category::Tabs => "▭",
        Category::Panes => "◫",
        Category::Agents => "❯",
        Category::Custom => "✦",
        Category::Discovered => "✧",
    }
}

fn truncate(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut out = text.chars().take(width - 1).collect::<String>();
    out.push('…');
    out
}

fn origin(binding: &Binding) -> &'static str {
    if binding.status == BindingStatus::Disabled {
        "unset"
    } else {
        match binding.source {
            BindingSource::Default => "default",
            BindingSource::Custom => "config",
        }
    }
}

fn badge_style(layer: KeyLayer, palette: Palette) -> Style {
    let fg = match layer {
        KeyLayer::Prefix => palette.accent,
        KeyLayer::Direct => palette.warning,
        KeyLayer::Unset => palette.muted,
    };
    Style::default()
        .fg(fg)
        .bg(palette.panel_alt)
        .add_modifier(Modifier::BOLD)
}

fn badge_span(text: &str, layer: KeyLayer, palette: Palette) -> Span<'static> {
    Span::styled(format!(" {text} "), badge_style(layer, palette))
}

fn separator(width: u16, palette: Palette) -> Line<'static> {
    Line::from(Span::styled(
        "─".repeat(width as usize),
        Style::default().fg(palette.panel_alt),
    ))
}

pub(crate) fn render_keys_view(frame: &mut Frame<'_>, area: Rect, app: &App, palette: Palette) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let groups = app.key_groups();
    let body = rows[4];
    let columns = pack_columns(&groups, body.height as usize);
    let max_visible = ((body.width + 1) / (MIN_COLUMN_WIDTH + 1)).max(1) as usize;
    let visible = columns.len().clamp(1, max_visible);
    let pages = columns.len().div_ceil(visible);
    let page = column_of(&columns, app.selected) / visible;

    render_title(frame, rows[0], app, palette, page, pages);
    frame.render_widget(Paragraph::new(separator(area.width, palette)), rows[1]);
    render_filter(frame, rows[2], app, palette);
    frame.render_widget(Paragraph::new(separator(area.width, palette)), rows[3]);
    render_columns(
        frame,
        body,
        app,
        &groups,
        &columns[page * visible..columns.len().min((page + 1) * visible)],
        visible,
        palette,
    );
    frame.render_widget(Paragraph::new(separator(area.width, palette)), rows[5]);
    render_details(frame, rows[6], app, palette);
    render_footer(frame, rows[7], app, palette);
}

fn render_title(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    palette: Palette,
    page: usize,
    pages: usize,
) {
    let mut left = vec![
        Span::styled(
            format!(" {}", app.title),
            Style::default()
                .fg(palette.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(palette.panel_alt)),
    ];
    match app.prefix.as_deref() {
        Some(prefix) => {
            left.push(Span::styled("prefix ", Style::default().fg(palette.muted)));
            left.push(Span::styled(
                prefix.to_string(),
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        None => left.push(Span::styled(
            "Herdr keybindings",
            Style::default().fg(palette.text),
        )),
    }

    let mut right = vec![
        Span::styled(
            app.navigation_view.label(),
            Style::default()
                .fg(palette.accent_2)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", Style::default().fg(palette.muted)),
        Span::styled(
            app.binding_view.label(),
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                " · {} / {} shown",
                app.filtered_bindings().len(),
                app.total_count()
            ),
            Style::default().fg(palette.muted),
        ),
    ];
    if pages > 1 {
        right.push(Span::styled(
            format!(" · page {}/{}", page + 1, pages),
            Style::default().fg(palette.warning),
        ));
    }
    if area.width >= COMPACT_WIDTH {
        right.push(Span::styled(
            "   Press <esc> to close ",
            Style::default().fg(palette.muted),
        ));
    }

    let right_line = Line::from(right);
    let right_width = (right_line.width() as u16).min(area.width);
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(right_width)])
        .split(area);
    frame.render_widget(Paragraph::new(Line::from(left)), split[0]);
    frame.render_widget(
        Paragraph::new(right_line).alignment(Alignment::Right),
        split[1],
    );
}

fn render_filter(frame: &mut Frame<'_>, area: Rect, app: &App, palette: Palette) {
    let mut left = vec![Span::styled(" ⌕ ", Style::default().fg(palette.accent))];
    if app.query.is_empty() {
        left.push(Span::styled(
            "Filter keys...",
            Style::default().fg(palette.muted),
        ));
    } else {
        left.push(Span::styled(
            app.query.clone(),
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        ));
        left.push(Span::styled("▏", Style::default().fg(palette.accent)));
    }

    let legend = if area.width >= COMPACT_WIDTH {
        Line::from(vec![
            badge_span("x", KeyLayer::Prefix, palette),
            Span::styled(" after prefix   ", Style::default().fg(palette.muted)),
            badge_span("alt+x", KeyLayer::Direct, palette),
            Span::styled(" direct   ", Style::default().fg(palette.muted)),
            badge_span("—", KeyLayer::Unset, palette),
            Span::styled(" unset ", Style::default().fg(palette.muted)),
        ])
    } else {
        Line::default()
    };
    let legend_width = (legend.width() as u16).min(area.width);
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(legend_width)])
        .split(area);
    frame.render_widget(Paragraph::new(Line::from(left)), split[0]);
    frame.render_widget(Paragraph::new(legend).alignment(Alignment::Right), split[1]);
}

fn render_columns(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    groups: &[KeyGroup],
    columns: &[Vec<Entry>],
    visible: usize,
    palette: Palette,
) {
    if groups.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " No bindings match that query.",
                Style::default().fg(palette.warning),
            ))),
            area,
        );
        return;
    }

    let flat = groups
        .iter()
        .flat_map(|group| group.bindings.iter().map(|(binding, _)| binding))
        .collect::<Vec<_>>();

    let mut constraints = Vec::new();
    for slot in 0..visible {
        if slot > 0 {
            constraints.push(Constraint::Length(1));
        }
        constraints.push(Constraint::Ratio(1, visible as u32));
    }
    let slots = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (slot, column) in columns.iter().enumerate() {
        if slot > 0 {
            let divider = (0..area.height)
                .map(|_| Line::from(Span::styled("│", Style::default().fg(palette.panel_alt))))
                .collect::<Vec<_>>();
            frame.render_widget(Paragraph::new(divider), slots[slot * 2 - 1]);
        }
        let rect = slots[slot * 2];
        let badge_width = column
            .iter()
            .filter_map(|entry| match entry {
                Entry::Item { index, .. } => Some(
                    key_badge(flat[*index])
                        .0
                        .chars()
                        .count()
                        .min(MAX_BADGE_WIDTH),
                ),
                _ => None,
            })
            .max()
            .unwrap_or(1);
        let lines = column
            .iter()
            .map(|entry| column_line(entry, &flat, app, badge_width, rect.width, palette))
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(lines), rect);
    }
}

fn column_line(
    entry: &Entry,
    flat: &[&Binding],
    app: &App,
    badge_width: usize,
    width: u16,
    palette: Palette,
) -> Line<'static> {
    match entry {
        Entry::Blank => Line::default(),
        Entry::Header {
            category,
            count,
            continued,
        } => {
            let mut spans = vec![
                Span::styled(
                    format!(" {} ", category_icon(*category)),
                    Style::default().fg(palette.warning),
                ),
                Span::styled(
                    category.title().to_uppercase(),
                    Style::default()
                        .fg(palette.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("  {count}"), Style::default().fg(palette.muted)),
            ];
            if *continued {
                spans.push(Span::styled(" (cont.)", Style::default().fg(palette.muted)));
            }
            Line::from(spans)
        }
        Entry::Item {
            index,
            last_in_group,
        } => {
            let binding = flat[*index];
            let selected = *index == app.selected;
            let (badge, layer) = key_badge(binding);
            let badge = truncate(&badge, MAX_BADGE_WIDTH);
            let pad = badge_width.saturating_sub(badge.chars().count());
            let connector = if *last_in_group { " └─" } else { " ├─" };
            let extra = binding.keys.len().saturating_sub(1);
            let suffix = if extra > 0 {
                format!(" +{extra}")
            } else {
                String::new()
            };
            let used = 3 + 1 + badge_width + 2 + 1 + suffix.chars().count();
            let label = truncate(&binding.label, (width as usize).saturating_sub(used));

            let row_style = if selected {
                Style::default()
                    .fg(palette.selected_text_color())
                    .bg(palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else if binding.status == BindingStatus::Disabled {
                Style::default().fg(palette.muted)
            } else {
                Style::default().fg(palette.text)
            };
            let mut spans = vec![
                Span::styled(connector, Style::default().fg(palette.panel_alt)),
                Span::raw(" "),
                badge_span(&badge, layer, palette),
                Span::raw(" ".repeat(pad)),
                Span::styled(" ", row_style),
                Span::styled(label, row_style),
            ];
            if !suffix.is_empty() {
                spans.push(Span::styled(
                    suffix,
                    if selected {
                        row_style
                    } else {
                        Style::default().fg(palette.muted)
                    },
                ));
            }
            if selected {
                spans.push(Span::styled(" ", row_style));
            }
            Line::from(spans)
        }
    }
}

fn render_details(frame: &mut Frame<'_>, area: Rect, app: &App, palette: Palette) {
    let line = match app.selected_binding() {
        Some(binding) => {
            let keys = if binding.keys.is_empty() {
                "unset".to_string()
            } else {
                binding.keys.join(", ")
            };
            let mut spans = vec![
                Span::styled(
                    format!(" {}", binding.label),
                    Style::default()
                        .fg(palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(keys, Style::default().fg(palette.accent_2)),
                Span::styled(
                    format!("  {}", origin(&binding)),
                    Style::default().fg(palette.muted),
                ),
            ];
            if binding.source == BindingSource::Custom && !binding.default_keys.is_empty() {
                spans.push(Span::styled(
                    format!(" (default {})", binding.default_keys.join(", ")),
                    Style::default().fg(palette.muted),
                ));
            }
            spans.push(Span::styled(
                format!("  {}", binding.hint),
                Style::default().fg(palette.text),
            ));
            Line::from(spans)
        }
        None => Line::from(Span::styled(
            " No bindings match that query.",
            Style::default().fg(palette.warning),
        )),
    };
    frame.render_widget(Paragraph::new(line), area);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App, palette: Palette) {
    let key = |text: &'static str| badge_span(text, KeyLayer::Prefix, palette);
    let label = |text: &'static str| Span::styled(text, Style::default().fg(palette.text));
    let full = Line::from(vec![
        Span::raw(" "),
        key("↑↓"),
        label(" move  "),
        key("←→"),
        label(" group  "),
        key("type"),
        label(" filter  "),
        key("tab"),
        label(" mode  "),
        key("ctrl+t"),
        label(" view  "),
        key("esc"),
        label(" close"),
    ]);
    // Narrow popups keep every control but drop the self-evident typing hint.
    let left = if full.width() <= area.width as usize {
        full
    } else {
        Line::from(vec![
            key("↑↓"),
            key("←→"),
            label(" nav "),
            key("tab"),
            label(" mode "),
            key("^t"),
            label(" view "),
            key("esc"),
            label(" close"),
        ])
    };
    let right = {
        Line::from(vec![Span::styled(
            format!(
                "{} · {} · {} assigned · {} unassigned ",
                app.config_path,
                app.theme_name,
                app.active_count(),
                app.unassigned_count()
            ),
            Style::default().fg(palette.muted),
        )])
    };
    // The summary only shows when it fits beside the controls with a gap.
    let right = if left.width() + 2 + right.width() <= area.width as usize {
        right
    } else {
        Line::default()
    };
    let right_width = right.width() as u16;
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(right_width)])
        .split(area);
    frame.render_widget(Paragraph::new(left), split[0]);
    frame.render_widget(Paragraph::new(right).alignment(Alignment::Right), split[1]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{effective_bindings, KeyValue, KeysSection};

    fn binding_for(keys: &KeysSection, action: &str) -> Binding {
        effective_bindings(keys)
            .into_iter()
            .find(|binding| binding.action == action)
            .unwrap()
    }

    #[test]
    fn badges_strip_prefix_and_classify_layers() {
        let keys = KeysSection {
            next_tab: Some(KeyValue::Many(vec!["prefix+n".into(), "alt+right".into()])),
            previous_tab: Some(KeyValue::One("alt+left".into())),
            focus_agent: Some(KeyValue::One(String::new())),
            ..Default::default()
        };
        assert_eq!(
            key_badge(&binding_for(&keys, "next_tab")),
            ("n".to_string(), KeyLayer::Prefix)
        );
        assert_eq!(
            key_badge(&binding_for(&keys, "previous_tab")),
            ("alt+left".to_string(), KeyLayer::Direct)
        );
        assert_eq!(
            key_badge(&binding_for(&keys, "focus_agent")).1,
            KeyLayer::Unset
        );
    }

    #[test]
    fn packing_covers_every_item_once_and_respects_height() {
        let app = App::new(
            effective_bindings(&KeysSection::default()),
            "x".into(),
            "terminal".into(),
        );
        let groups = app.key_groups();
        let total = groups
            .iter()
            .map(|group| group.bindings.len())
            .sum::<usize>();
        for height in [2usize, 3, 5, 10, 20, 80] {
            let columns = pack_columns(&groups, height);
            let mut seen = columns
                .iter()
                .flatten()
                .filter_map(|entry| match entry {
                    Entry::Item { index, .. } => Some(*index),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(columns.iter().all(|column| column.len() <= height.max(2)));
            assert!(columns
                .iter()
                .all(|column| !matches!(column.last(), Some(Entry::Header { .. }))));
            seen.sort_unstable();
            assert_eq!(seen, (0..total).collect::<Vec<_>>(), "height {height}");
        }
    }

    #[test]
    fn spilled_groups_repeat_their_header() {
        let app = App::new(
            effective_bindings(&KeysSection::default()),
            "x".into(),
            "terminal".into(),
        );
        let columns = pack_columns(&app.key_groups(), 6);
        assert!(columns
            .iter()
            .skip(1)
            .all(|column| matches!(column.first(), Some(Entry::Header { .. }))));
        assert!(columns.iter().flatten().any(|entry| matches!(
            entry,
            Entry::Header {
                continued: true,
                ..
            }
        )));
    }

    #[test]
    fn truncate_adds_ellipsis_only_when_needed() {
        assert_eq!(truncate("abc", 3), "abc");
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate("abc", 0), "");
    }
}
