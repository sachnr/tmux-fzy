use std::collections::BinaryHeap;

use ratatui::prelude::*;
use ratatui::widgets::{Block, List, ListDirection, ListItem, Padding, Paragraph};

use crate::config::Colors;
use crate::tui::PathItem;

pub fn get_input_bar<'a>(input: &'a String, colors: &'a Colors) -> Paragraph<'a> {
    let inputs: Vec<Span<'a>> = vec![
        Span::styled("  ", Style::default().fg(colors.active)),
        Span::styled(input, Style::default().fg(colors.fg)),
    ];
    let line = Line::from(inputs);
    Paragraph::new(line)
        .style(Style::default().fg(colors.fg))
        .block(
            Block::default()
                .style(Style::default().fg(colors.active))
                .padding(Padding::new(0, 0, 0, 0)),
        )
}

pub fn get_list<'a>(
    items: &'a BinaryHeap<PathItem>,
    rows: u16,
    curr_row: Option<usize>,
    colors: &'a Colors,
) -> List<'a> {
    let iter = items.iter().enumerate().map(move |(i, item)| {
        let curr_row = curr_row.unwrap_or(0);
        let upper_index = curr_row.saturating_sub(rows as usize);

        // only highlight rows that are visible
        if i >= upper_index && i < curr_row + rows as usize {
            let mut spans = Vec::new();
            let mut style = Style::default().fg(colors.fg);
            if i == curr_row {
                style.fg = Some(colors.active);
                style.add_modifier = Modifier::BOLD;
            }
            let mut curr_pos: usize = 0;
            for ind in &item.indices {
                spans.push(Span::styled(&item.path[curr_pos..*ind], style));
                curr_pos = ind + 1;
                spans.push(Span::styled(
                    &item.path[*ind..curr_pos],
                    style.fg(colors.selection),
                ));
            }
            if curr_pos < item.path.len() - 1 {
                spans.push(Span::styled(&item.path[curr_pos..item.path.len()], style));
            }
            let line = Line::from(spans);
            ListItem::new(line)
        } else {
            ListItem::new(item.path)
        }
    });

    List::new(iter)
        .block(
            Block::default()
                .title("Results")
                .style(Style::default().fg(colors.active)),
        )
        .highlight_symbol("▪ ")
        .direction(ListDirection::TopToBottom)
}

pub fn get_total_item_no(total_len: usize, curr_len: usize, colors: &Colors) -> Paragraph<'_> {
    let text = format!("{}/{}", curr_len, total_len);
    Paragraph::new(text).block(Block::default().fg(colors.selection))
}
