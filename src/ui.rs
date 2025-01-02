use crate::{globe, state::*};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    style::{Color, Modifier, Style},
    symbols,
    widgets::{canvas::*, Block, Borders, Clear, Paragraph, Row, Table, TableState},
    Frame,
};
use ratatui_image::{thread::ThreadImage, Resize};

fn _render_filename(app: &mut Application, frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new(app.path_to_image.display().to_string())
            .style(Style::new().italic().bold().green())
            .block(
                Block::new()
                    .title("Filename")
                    .title_style(Style::new().bold())
                    .border_set(symbols::border::ROUNDED)
                    .borders(Borders::ALL),
            ),
        area,
    )
}

fn render_metadata_table(
    app: &mut Application,
    frame: &mut Frame,
    table_state: &mut TableState,
    area: Rect,
) {
    // let widths = [Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)];
    let widths = Constraint::from_mins([100, 100]);
    let exif_table = Table::new(app.process_rows(frame.area().width), widths).column_spacing(1);

    frame.render_stateful_widget(
        exif_table
            .block(
                Block::new()
                    .title("Image Metadata")
                    .title_style(Style::new().bold())
                    .border_set(symbols::border::ROUNDED)
                    .borders(Borders::TOP | Borders::RIGHT | Borders::LEFT), // .padding(Padding::uniform(1)),
            )
            .header(Row::new(vec!["Tag", "Data"]).bold().underlined())
            .highlight_style(
                Style::new()
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
        top_left: symbols::line::ROUNDED.vertical_right,
        top_right: symbols::line::ROUNDED.vertical_left,
        ..symbols::border::ROUNDED
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
                    .borders(Borders::RIGHT | Borders::LEFT | Borders::TOP),
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

                // Print GPS Coordinates in Bottom-Left Corner
                ctx.print(0 as f64, 0 as f64, app.gps_info.to_string());

                // default character size is 4 by 8
                for i in 0..size_y {
                    for j in 0..size_x {
                        let translated_i = 50 - i;
                        let translated_j = j as f64 + 12.5;
                        match globe_canvas.matrix[i][j] {
                            ' ' => ctx.print(translated_j as f64, translated_i as f64, " "),
                            '.' | ':' | ';' => {
                                let x = globe_canvas.matrix[i][j].to_string().dim();
                                ctx.print(translated_j as f64, translated_i as f64, x);
                            }
                            x => {
                                // Only useful when there is no z-axis panning going on
                                let long_lat_color = if app.has_gps
                                    && !app.should_rotate
                                    && i == (size_y / 2) - 1
                                    && j == (size_x / 2) - 1
                                {
                                    x.to_string().red().bold().rapid_blink()
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

fn render_image(app: &mut Application, frame: &mut Frame, area: Rect) {
    let collapsed_top_border_set = symbols::border::Set {
        top_left: symbols::line::NORMAL.vertical_right,
        top_right: symbols::line::NORMAL.vertical_left,
        ..symbols::border::ROUNDED
    };

    let block = Block::default()
        .title("Thumbnail")
        .title_style(Style::new().bold())
        .border_set(collapsed_top_border_set)
        .borders(Borders::RIGHT | Borders::LEFT | Borders::TOP);

    let rect = centered_rect(block.inner(area), 50, 100);
    let image = ThreadImage::default().resize(Resize::Fit(None));

    frame.render_stateful_widget(image, rect, &mut app.async_state);
    frame.render_widget(block.clone(), area);
}

fn render_status_msg(app: &mut Application, frame: &mut Frame, area: Rect) {
    let collapsed_top_border_set = symbols::border::Set {
        top_left: symbols::line::ROUNDED.vertical_right,
        top_right: symbols::line::ROUNDED.vertical_left,
        // bottom_left: symbols::line::NORMAL.horizontal_up,
        ..symbols::border::ROUNDED
    };
    frame.render_widget(
        Paragraph::new(app.status_msg.clone()).block(
            Block::new()
                .title("Status")
                .title_style(Style::new().bold())
                .borders(Borders::ALL)
                .border_set(collapsed_top_border_set),
        ),
        area,
    );
}

fn render_keybind_popup(app: &mut Application, frame: &mut Frame) {
    let pop_area = centered_rect(frame.area(), 50, 50);
    let widths = [Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)];
    let keybind_table = Table::new(app.keybind_rows(), widths).column_spacing(1);
    frame.render_widget(Clear, pop_area);
    frame.render_widget(
        keybind_table.block(
            Block::new()
                .title("Keybinds")
                .title_style(Style::new().bold())
                .borders(Borders::ALL)
                .border_set(symbols::border::ROUNDED),
        ),
        pop_area,
    )
}

pub fn view(app: &mut Application, frame: &mut Frame, table_state: &mut TableState) {
    if app.show_mini {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Max(45),
                Constraint::Max(50),
                Constraint::Max(5),
            ])
            .split(frame.area());
        render_metadata_table(app, frame, table_state, layout[0]);
        match app.render_state {
            RenderState::Globe => render_globe(app, frame, layout[1]),
            RenderState::Thumbnail => render_image(app, frame, layout[1]),
        };
        render_status_msg(app, frame, layout[2]);
    } else {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                // Constraint::Max(5),
                Constraint::Max(95),
                Constraint::Max(5),
            ])
            .split(frame.area());
        render_metadata_table(app, frame, table_state, layout[0]);
        render_status_msg(app, frame, layout[1]);
    }

    if app.show_keybinds {
        render_keybind_popup(app, frame);
    }
}

/// # Usage
///
/// ```rust
/// let rect = centered_rect(f.area(), 50, 50);
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
