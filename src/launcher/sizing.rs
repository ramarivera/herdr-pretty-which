use ratatui::text::Line;

/// Count screen rows occupied by text at the available terminal width.
pub(super) fn wrapped_rows(text: &str, width: u16) -> usize {
    let width = usize::from(width.max(1));
    text.split('\n')
        .map(|line| {
            let mut rows = 1usize;
            let mut used = 0usize;
            for word in line.split_inclusive(char::is_whitespace) {
                let cells = Line::from(word).width();
                if used > 0 && cells > width.saturating_sub(used) {
                    rows += 1;
                    used = 0;
                }
                if cells > width {
                    rows += (cells - 1) / width;
                    used = ((cells - 1) % width) + 1;
                } else {
                    used += cells;
                }
            }
            rows
        })
        .sum()
}

pub(super) fn wrap_input(input: &str, width: u16) -> Vec<String> {
    let width = usize::from(width.saturating_sub(2).max(1));
    let mut lines = vec![String::new()];
    let mut used = 0;
    for ch in format!("> {input}").chars() {
        let cells = Line::from(ch.to_string()).width();
        if used > 0 && used + cells > width {
            lines.push(String::new());
            used = 0;
        }
        lines.last_mut().expect("one line exists").push(ch);
        used += cells;
    }
    lines
}

pub(super) fn input_height(lines: usize, available_height: u16) -> u16 {
    let desired = lines.saturating_add(2).min(u16::MAX as usize) as u16;
    desired
        .min(available_height.saturating_sub(2))
        .max(3.min(available_height))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_grows_with_display_width_and_respects_viewport() {
        assert_eq!(input_height(wrap_input("", 20).len(), 20), 3);
        assert_eq!(
            input_height(wrap_input("abcdefghijklmnopq", 20).len(), 20),
            4
        );
        assert_eq!(
            input_height(wrap_input("界界界界界界界界界", 20).len(), 20),
            4
        );
        assert_eq!(input_height(wrap_input(&"x".repeat(1000), 20).len(), 10), 8);
    }
}
