use crate::app::{App, NavigationViewMode, TreeRow, TreeRowKind};
use crate::model::{Binding, BindingSource, BindingStatus, Category};
use crate::theme::Palette;
use anyhow::Result;
use ratatui::backend::TestBackend;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Row, Table, Wrap};
use ratatui::{Frame, Terminal};

pub fn render_app(frame: &mut Frame<'_>, app: &App, palette: Palette) {
    render_app_with_embed_mode(frame, app, palette, true);
}

fn render_app_with_embed_mode(
    frame: &mut Frame<'_>,
    app: &App,
    palette: Palette,
    embedded_in_herdr: bool,
) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    let content_area = if embedded_in_herdr {
        area
    } else {
        let outer = Block::default()
            .title(Line::from(vec![
                Span::styled(
                    " ",
                    Style::default()
                        .fg(palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &app.title,
                    Style::default()
                        .fg(palette.text)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " — mnemonic training wheels ",
                    Style::default().fg(palette.muted),
                ),
            ]))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.accent));
        let inner = outer.inner(area);
        frame.render_widget(outer, area);
        inner
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(5),
        ])
        .split(content_area);

    render_header(frame, chunks[0], app, palette);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(72), Constraint::Length(36)])
        .split(chunks[1]);

    let side = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(8)])
        .split(body[1]);

    render_bindings(frame, body[0], app, palette);
    render_details(frame, side[0], app, palette);
    render_categories(frame, side[1], app, palette);
    render_footer(frame, chunks[2], palette);
}

pub fn render_to_test_backend(
    app: &App,
    palette: Palette,
    width: u16,
    height: u16,
) -> Result<TestBackend> {
    render_to_test_backend_with_embed_mode(app, palette, width, height, false)
}

pub fn render_embedded_to_test_backend(
    app: &App,
    palette: Palette,
    width: u16,
    height: u16,
) -> Result<TestBackend> {
    render_to_test_backend_with_embed_mode(app, palette, width, height, true)
}

fn render_to_test_backend_with_embed_mode(
    app: &App,
    palette: Palette,
    width: u16,
    height: u16,
    embedded_in_herdr: bool,
) -> Result<TestBackend> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| render_app_with_embed_mode(frame, app, palette, embedded_in_herdr))?;
    Ok(terminal.backend().clone())
}

pub fn render_to_string(app: &App, palette: Palette, width: u16, height: u16) -> Result<String> {
    let backend = render_to_test_backend(app, palette, width, height)?;
    Ok(buffer_to_string(backend.buffer().area, backend.buffer()))
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App, palette: Palette) {
    let mut spans = vec![
        Span::styled("Config ", Style::default().fg(palette.muted)),
        Span::styled(&app.config_path, Style::default().fg(palette.text)),
        Span::raw("   "),
        Span::styled("Theme ", Style::default().fg(palette.muted)),
        Span::styled(&app.theme_name, Style::default().fg(palette.accent_2)),
    ];
    spans.push(Span::raw("   "));
    spans.push(Span::styled(
        format!(
            "{} assigned · {} unassigned",
            app.active_count(),
            app.unassigned_count()
        ),
        Style::default().fg(palette.success),
    ));

    let query = if app.query.is_empty() {
        "type to fuzzy-filter".to_string()
    } else {
        app.query.clone()
    };
    let text = vec![
        Line::from(spans),
        Line::from(vec![
            Span::styled("Search ", Style::default().fg(palette.muted)),
            Span::styled(
                format!("› {query}"),
                Style::default()
                    .fg(if app.query.is_empty() {
                        palette.muted
                    } else {
                        palette.accent
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled("Mode ", Style::default().fg(palette.muted)),
            Span::styled(
                app.binding_view.label(),
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled("View ", Style::default().fg(palette.muted)),
            Span::styled(
                app.navigation_view.label(),
                Style::default()
                    .fg(palette.accent_2)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled("Shown ", Style::default().fg(palette.muted)),
            Span::styled(
                format!(
                    "{} / {} bindings",
                    app.filtered_bindings().len(),
                    app.total_count()
                ),
                Style::default().fg(palette.text),
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(text).block(panel("Session", palette)), area);
}

fn render_categories(frame: &mut Frame<'_>, area: Rect, app: &App, palette: Palette) {
    let counts = app.category_counts();
    let items = Category::ALL
        .iter()
        .map(|category| {
            let count = counts.get(category).copied().unwrap_or(0);
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<11}", category.title()),
                    Style::default().fg(palette.text),
                ),
                Span::styled(format!("{:>2}", count), Style::default().fg(palette.accent)),
            ]))
        })
        .collect::<Vec<_>>();
    frame.render_widget(List::new(items).block(panel("Categories", palette)), area);
}

fn render_bindings(frame: &mut Frame<'_>, area: Rect, app: &App, palette: Palette) {
    match app.navigation_view {
        NavigationViewMode::List => render_list_bindings(frame, area, app, palette),
        NavigationViewMode::Tree => render_tree_bindings(frame, area, app, palette),
    }
}

fn render_list_bindings(frame: &mut Frame<'_>, area: Rect, app: &App, palette: Palette) {
    let filtered = app.filtered_bindings();
    let visible_rows = area.height.saturating_sub(3).max(1) as usize;
    let selected = app.selected.min(filtered.len().saturating_sub(1));
    let offset = selected.saturating_sub(visible_rows.saturating_sub(1));
    let rows = filtered
        .into_iter()
        .enumerate()
        .skip(offset)
        .take(visible_rows)
        .map(|(index, (binding, score))| binding_row(index == selected, binding, score, palette))
        .collect::<Vec<_>>();

    let table = Table::new(
        rows,
        [
            Constraint::Length(18),
            Constraint::Length(22),
            Constraint::Min(16),
            Constraint::Length(8),
        ],
    )
    .header(
        Row::new(["Action", "Keys", "Hint", "Origin"]).style(Style::default().fg(palette.muted)),
    )
    .block(panel("Bindings · List", palette))
    .column_spacing(1);
    frame.render_widget(table, area);
}

fn render_tree_bindings(frame: &mut Frame<'_>, area: Rect, app: &App, palette: Palette) {
    let rows = app.visible_tree_rows();
    let selected_visible = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.selectable)
        .nth(app.selected)
        .map(|(index, _)| index)
        .unwrap_or(0);
    let visible_rows = area.height.saturating_sub(2).max(1) as usize;
    let offset = selected_visible.saturating_sub(visible_rows.saturating_sub(1));
    let mut selectable_index = 0usize;
    let items = rows
        .into_iter()
        .map(|row| {
            let selected = if row.selectable {
                let selected = selectable_index == app.selected;
                selectable_index += 1;
                selected
            } else {
                false
            };
            tree_item(row, selected, app.query.trim(), palette)
        })
        .skip(offset)
        .take(visible_rows)
        .collect::<Vec<_>>();

    frame.render_widget(
        List::new(items).block(panel("Bindings · Tree", palette)),
        area,
    );
}

fn binding_row(
    selected: bool,
    binding: Binding,
    score: Option<i64>,
    palette: Palette,
) -> Row<'static> {
    let style = if selected {
        Style::default()
            .fg(palette.selected_text_color())
            .bg(palette.accent)
            .add_modifier(Modifier::BOLD)
    } else if binding.status == BindingStatus::Disabled {
        Style::default().fg(palette.muted)
    } else {
        Style::default().fg(palette.text)
    };
    let label = binding.label.clone();
    let keys = keys_for(&binding);
    let source = origin_for(&binding);
    let hint = score
        .map(|score| format!("{} · match {score}", binding.hint))
        .unwrap_or(binding.hint);
    Row::new([label, keys, hint, source.to_string()]).style(style)
}

fn tree_item(row: TreeRow, selected: bool, query: &str, palette: Palette) -> ListItem<'static> {
    let base_style = if selected {
        Style::default()
            .fg(palette.selected_text_color())
            .bg(palette.accent)
            .add_modifier(Modifier::BOLD)
    } else if row.context_only
        || row
            .binding
            .as_ref()
            .is_some_and(|binding| binding.status == BindingStatus::Disabled)
    {
        Style::default().fg(palette.muted)
    } else {
        Style::default().fg(palette.text)
    };
    let mut spans = vec![Span::raw("  ".repeat(row.depth))];
    let glyph = match row.kind {
        TreeRowKind::Group if row.expanded => "▾ ",
        TreeRowKind::Group => "▸ ",
        TreeRowKind::Binding => "• ",
    };
    spans.push(Span::styled(glyph, base_style));
    spans.extend(highlighted_label(
        &row.label, query, base_style, selected, palette,
    ));

    if let Some(binding) = row.binding.as_ref() {
        spans.push(Span::styled("  ", base_style));
        let metadata_style = |color| {
            if selected {
                Style::default()
                    .fg(Palette::selected_text_color_for(color, palette.accent))
                    .bg(palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            }
        };
        spans.push(Span::styled(
            keys_for(binding),
            metadata_style(palette.accent_2),
        ));
        spans.push(Span::styled("  ", base_style));
        spans.push(Span::styled(
            origin_for(binding),
            metadata_style(palette.muted),
        ));
        if row.score.is_some() {
            spans.push(Span::styled("  match", metadata_style(palette.muted)));
        }
    }

    ListItem::new(Line::from(spans)).style(base_style)
}

fn highlighted_label(
    label: &str,
    query: &str,
    base_style: Style,
    selected: bool,
    palette: Palette,
) -> Vec<Span<'static>> {
    if query.is_empty() {
        return vec![Span::styled(label.to_string(), base_style)];
    }
    let label_lower = label.to_lowercase();
    let query_lower = query.to_lowercase();
    let Some(start) = label_lower.find(&query_lower) else {
        return vec![Span::styled(label.to_string(), base_style)];
    };
    let end = start + query_lower.len();
    let highlight_style = if selected {
        Style::default()
            .fg(palette.selected_text_color())
            .bg(palette.accent)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default()
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    };
    vec![
        Span::styled(label[..start].to_string(), base_style),
        Span::styled(label[start..end].to_string(), highlight_style),
        Span::styled(label[end..].to_string(), base_style),
    ]
}

fn keys_for(binding: &Binding) -> String {
    if binding.keys.is_empty() {
        "—".to_string()
    } else {
        binding.keys.join(", ")
    }
}

fn origin_for(binding: &Binding) -> &'static str {
    if binding.status == BindingStatus::Disabled {
        "unset"
    } else {
        match binding.source {
            BindingSource::Default => "default",
            BindingSource::Custom => "config",
        }
    }
}

fn render_details(frame: &mut Frame<'_>, area: Rect, app: &App, palette: Palette) {
    let selected = app.selected_binding();
    let selected_group = (app.navigation_view == NavigationViewMode::Tree)
        .then(|| app.selected_tree_row())
        .flatten()
        .filter(|row| row.kind == TreeRowKind::Group);
    let lines = if let Some(group) = selected_group {
        vec![
            Line::from(Span::styled(
                group.label,
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Type    ", Style::default().fg(palette.muted)),
                Span::raw("group"),
            ]),
            Line::from(vec![
                Span::styled("Path    ", Style::default().fg(palette.muted)),
                Span::raw(group.path.join(" / ")),
            ]),
            Line::from(vec![
                Span::styled("State   ", Style::default().fg(palette.muted)),
                Span::raw(if group.expanded {
                    "expanded"
                } else {
                    "collapsed"
                }),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Use ←/→ to collapse, expand, or enter the group.",
                Style::default().fg(palette.text),
            )),
        ]
    } else if let Some(binding) = selected {
        vec![
            Line::from(Span::styled(
                binding.label,
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Action  ", Style::default().fg(palette.muted)),
                Span::raw(binding.action),
            ]),
            Line::from(vec![
                Span::styled("Keys    ", Style::default().fg(palette.muted)),
                Span::raw(if binding.keys.is_empty() {
                    "disabled".to_string()
                } else {
                    binding.keys.join(", ")
                }),
            ]),
            Line::from(vec![
                Span::styled("Default ", Style::default().fg(palette.muted)),
                Span::raw(if binding.default_keys.is_empty() {
                    "unset".to_string()
                } else {
                    binding.default_keys.join(", ")
                }),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                binding.hint,
                Style::default().fg(palette.text),
            )),
        ]
    } else {
        vec![Line::from(Span::styled(
            "No bindings match that query.",
            Style::default().fg(palette.warning),
        ))]
    };
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel("Details", palette)),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, palette: Palette) {
    let lines = vec![
        Line::from(vec![
            Span::styled("↑/↓", Style::default().fg(palette.accent)),
            Span::raw(" move   "),
            Span::styled("type", Style::default().fg(palette.accent)),
            Span::raw(" filter   "),
            Span::styled("backspace", Style::default().fg(palette.accent)),
            Span::raw(" edit   "),
            Span::styled("tab/shift-tab", Style::default().fg(palette.accent)),
            Span::raw(" mode   "),
            Span::styled("ctrl+t", Style::default().fg(palette.accent)),
            Span::raw(" view   "),
            Span::styled("←/→", Style::default().fg(palette.accent)),
            Span::raw(" tree   "),
            Span::styled("esc/q", Style::default().fg(palette.accent)),
            Span::raw(" close"),
        ]),
        Line::from(Span::styled("Tip: learn Herdr's native map first; direct chords are optional seasoning, not the main course.", Style::default().fg(palette.muted))),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(panel("Muscle memory", palette)),
        area,
    );
}

fn panel<'a>(title: &'a str, palette: Palette) -> Block<'a> {
    Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.panel_alt))
        .style(Style::default().bg(palette.panel).fg(palette.text))
}

fn buffer_to_string(area: Rect, buffer: &ratatui::buffer::Buffer) -> String {
    let area = area.inner(Margin {
        vertical: 0,
        horizontal: 0,
    });
    let mut lines = Vec::new();
    for y in area.top()..area.bottom() {
        let mut line = String::new();
        for x in area.left()..area.right() {
            line.push_str(buffer[(x, y)].symbol());
        }
        lines.push(line.trim_end().to_string());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{effective_bindings, KeysSection};
    use crate::theme::{Palette, ThemeConfig};

    #[test]
    fn render_contains_core_labels() {
        let app = App::new(
            effective_bindings(&KeysSection::default()),
            "config.toml".into(),
            "terminal".into(),
        );
        let text =
            render_to_string(&app, Palette::from_theme(&ThemeConfig::default()), 100, 30).unwrap();
        assert!(text.contains("Herdr Pretty Which"));
        assert!(text.contains("Help"));
        assert!(text.contains("Core"));
    }

    #[test]
    fn border_style_uses_theme_accent_color() {
        let app = App::new(
            effective_bindings(&KeysSection::default()),
            "config.toml".into(),
            "catppuccin".into(),
        );
        let palette = Palette::from_theme(&ThemeConfig {
            name: Some("catppuccin".to_string()),
            custom: crate::theme::ThemeCustom {
                mauve: Some("#8839ef".to_string()),
                ..Default::default()
            },
        });
        let backend = render_to_test_backend(&app, palette, 100, 30).unwrap();
        assert_eq!(backend.buffer()[(0, 0)].style().fg, Some(palette.accent));
    }

    #[test]
    fn herdr_embedded_mode_does_not_draw_outer_border() {
        let app = App::new(
            effective_bindings(&KeysSection::default()),
            "config.toml".into(),
            "terminal".into(),
        );
        let palette = Palette::from_theme(&ThemeConfig::default());
        let backend = render_embedded_to_test_backend(&app, palette, 100, 30).unwrap();
        let first_line = (0..100)
            .map(|x| backend.buffer()[(x, 0)].symbol())
            .collect::<String>();
        assert!(first_line.contains("Session"));
        assert!(!first_line.contains("Herdr Pretty Which — mnemonic training wheels"));
    }

    #[test]
    fn selected_tree_metadata_uses_contrast_safe_colors() {
        let mut app = App::new(
            effective_bindings(&KeysSection::default()),
            "config.toml".into(),
            "catppuccin-latte".into(),
        );
        app.set_query("workspace picker");
        let palette = Palette::from_theme(&ThemeConfig {
            name: Some("catppuccin-latte".to_string()),
            custom: Default::default(),
        });
        let backend = render_embedded_to_test_backend(&app, palette, 100, 18).unwrap();
        let buffer = backend.buffer();
        let expected_key_color = Palette::selected_text_color_for(palette.accent_2, palette.accent);
        let expected_origin_color = Palette::selected_text_color_for(palette.muted, palette.accent);

        let mut selected_cells = buffer
            .content()
            .iter()
            .filter(|cell| cell.style().bg == Some(palette.accent) && cell.symbol() != " ")
            .peekable();
        assert!(selected_cells.peek().is_some(), "expected a selected row");

        for cell in selected_cells {
            assert_ne!(cell.style().fg, Some(palette.accent_2));
            assert_ne!(cell.style().fg, Some(palette.muted));
            assert!(
                matches!(
                    cell.style().fg,
                    Some(color) if color == expected_key_color || color == expected_origin_color || color == palette.selected_text_color()
                ),
                "selected row cell used an unexpected foreground color: {:?}",
                cell.style().fg
            );
        }
    }
}
