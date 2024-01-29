use crate::globe::{self};
use crate::state::*;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    style::{Color, Modifier, Style},
    symbols,
    widgets::{canvas::*, Block, Borders, Clear, Padding, Row, Table, TableState},
    Frame,
};
use ratatui_image::{Image, StatefulImage};

fn render_metadata_table(
    app: &mut Application,
    frame: &mut Frame,
    table_state: &mut TableState,
    area: Rect,
) {
    let widths = [Constraint::Length(30), Constraint::Length(70)];
    let exif_table = Table::new(app.process_rows(), widths).column_spacing(1);

    frame.render_stateful_widget(
        exif_table
            .block(
                Block::new()
                    .title("Metadata")
                    .title_style(Style::new().bold())
                    .border_set(symbols::border::PLAIN)
                    .borders(Borders::TOP | Borders::RIGHT | Borders::LEFT)
                    .padding(Padding::uniform(1)),
            )
            .header(Row::new(vec!["Tag", "Data"]).bold().underlined())
            // .style(Style::new().bold())
            .highlight_style(
                Style::new()
                    .cyan()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray),
            )
            .highlight_symbol("> "),
        area,
        // centered_rect(layout[0], 100, 100),
        table_state,
    );
}

fn render_globe(app: &mut Application, frame: &mut Frame, area: Rect) {
    let collapsed_top_border_set = symbols::border::Set {
        top_left: symbols::line::NORMAL.vertical_right,
        top_right: symbols::line::NORMAL.vertical_left,
        // bottom_left: symbols::line::NORMAL.horizontal_up,
        ..symbols::border::PLAIN
    };

    frame.render_widget(
        Canvas::default()
            .block(
                Block::default()
                    .title(if app.has_gps {
                        "Image Location"
                    } else {
                        "Globe"
                    })
                    .title_style(Style::new().bold())
                    .border_set(collapsed_top_border_set)
                    .borders(Borders::ALL),
            )
            .x_bounds([0., 100.])
            .y_bounds([0., 50.])
            .paint(|ctx| {
                // Globe Width should be 3/4 of the width of the frame to look spherical
                // let adjusted_width = (layout[1].width as f64 * 0.75) as u16;
                // println!("{:?}", adjusted_width);
                ctx.layer();
                let mut globe_canvas = globe::Canvas::new(75, 50, Some((1, 1)));
                globe_canvas.clear();
                app.globe.render_sphere(&mut globe_canvas);
                let (size_x, size_y) = globe_canvas.get_size();
                // default character size is 4 by 8
                for i in 0..size_y {
                    for j in 0..size_x {
                        let translated_i = 50 - i;
                        let translated_j = j as f64 + 12.5;
                        match globe_canvas.matrix[i][j] {
                            ' ' => ctx.print(translated_j as f64, translated_i as f64, " "),
                            x => {
                                // Only useful when there is no z-axis panning going on
                                let long_lat_color = if app.has_gps
                                    && i == (size_y / 2) - 1
                                    && j == (size_x / 2) - 1
                                {
                                    x.to_string().red().bold().slow_blink()
                                } else {
                                    x.to_string().into()
                                };
                                ctx.print(translated_j as f64, translated_i as f64, long_lat_color)
                                // ctx.print(translated_j as f64, translated_i as f64, x.to_string())
                            }
                        }
                    }
                }
            }),
        area, // centered_rect(layout[1], 80, 80),
    );
}

fn render_keybind_popup(app: &mut Application, frame: &mut Frame) {
    let pop_area = centered_rect(frame.size(), 50, 50);
    let widths = [Constraint::Length(10), Constraint::Length(90)];
    let keybind_table = Table::new(app.keybind_rows(), widths).column_spacing(1);
    frame.render_widget(Clear, pop_area);
    frame.render_widget(
        keybind_table.block(
            Block::new()
                .title("Keybinds")
                .title_style(Style::new().bold())
                .borders(Borders::ALL),
        ),
        pop_area,
    )
}

// fn render_image(app: &mut Application, frame: &mut Frame, area: Rect) {
//     let collapsed_top_border_set = symbols::border::Set {
//         top_left: symbols::line::NORMAL.vertical_right,
//         top_right: symbols::line::NORMAL.vertical_left,
//         // bottom_left: symbols::line::NORMAL.horizontal_up,
//         ..symbols::border::PLAIN
//     };
//
//     // let within_image_block = Layout::default().direction(Direction::Vertical).constraints([
//     // 	Constraint::Percentage(frame.size())
//     // ]);
//
//     let block = Block::default()
//         .title("Thumbnail")
//         .title_style(Style::new().bold())
//         .border_set(collapsed_top_border_set)
//         .borders(Borders::ALL);
//     frame.render_widget(block.clone(), area);
//
//     let rect = centered_rect(block.inner(area), 50, 100);
//
//     let image = StatefulImage::new(None).resize(ratatui_image::Resize::Fit);
//     // let image = Image::new(app.image_static.as_ref());
//
//     frame.render_stateful_widget(image, rect, &mut app.image_static);
//
//     // frame.render_widget(image, area)
// }

pub fn view(app: &mut Application, frame: &mut Frame, table_state: &mut TableState) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(frame.size());

    render_metadata_table(app, frame, table_state, layout[0]);
    render_globe(app, frame, layout[1]);
    // render_image(app, frame, layout[1]);

    if app.show_keybinds {
        render_keybind_popup(app, frame);
    }
}

/// # Usage
///
/// ```rust
/// let rect = centered_rect(f.size(), 50, 50);
/// ```
fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
