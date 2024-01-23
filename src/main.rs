use anyhow::Result;
use bresson::globe::{self, Globe};
use bresson::state::*;
use std::path::Path;
use tui::restore_terminal;

use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    style::{Color, Modifier, Style},
    symbols,
    widgets::{canvas::*, Block, Borders, Row, Table, TableState},
    Frame,
};

fn main() -> Result<()> {
    if std::env::args().len() < 2 {
        std::process::exit(1);
    }
    let image_arg = std::env::args().nth(1).unwrap();

    // CLI Mode
    let app_mode = match std::env::args().nth(2) {
        Some(second) => {
            if second.eq("-c") {
                AppMode::CommandLine
            } else {
                AppMode::Interactive
            }
        }
        None => AppMode::Interactive,
    };

    let image_file = Path::new(&image_arg);
    if image_file.is_file() {
        println!("Image: {}\n", image_file.display());
    } else {
        println!("Image not present");
        return Ok(());
    }
    let cam_zoom = 1.5;
    let cam_xy = 0.;
    let cam_z = 0.;
    let mut globe = Globe::new(1., 0., false);
    globe.camera.update(cam_zoom, cam_xy, cam_z);
    // globe.angle += 1.1;
    let mut metadata = Application::new(image_file, globe, app_mode)?;
    let mut table_state = TableState::new().with_selected(Some(0));

    match app_mode {
        AppMode::CommandLine => {
            // Print out the Exif Data in the CLI
            metadata.clear_fields();
            metadata.save_state()?;
            Ok(())
        }
        AppMode::Interactive => {
            tui::install_panic_hook();
            let mut terminal = tui::init_terminal()?;
            terminal.clear()?;

            if metadata.has_gps {
                // Go to the coordinates extracted from the input
                metadata.transform_coordinates();
            }

            loop {
                terminal.draw(|frame| view(&mut metadata, frame, &mut table_state))?;
                if event::poll(std::time::Duration::from_millis(16))? {
                    if let event::Event::Key(key) = event::read()? {
                        if key.kind == KeyEventKind::Press {
                            match key.code {
                                KeyCode::Char(c) => match c {
                                    'o' | 'O' => {
                                        // Show Original Data
                                        metadata.modified_fields = metadata.original_fields.clone();
                                    }
                                    'q' => break,
                                    'r' => {
                                        // Only randomize the selected element based on table state
                                        match table_state.selected() {
                                            Some(index) => metadata.randomize(index),
                                            None => {}
                                        }
                                    }
                                    'R' => {
                                        // Randomize all fields (generalize over the individual field)
                                        metadata.randomize_all()
                                    }
                                    'c' | 'C' => metadata.clear_fields(),
                                    's' | 'S' => {
                                        // Save the state into a file copy
                                        metadata.save_state()?
                                    }
                                    '+' => metadata.camera_zoom_increase(),
                                    '-' => metadata.camera_zoom_decrease(),
                                    _ => {}
                                },
                                KeyCode::Esc => break,
                                KeyCode::Down => match table_state.selected() {
                                    Some(i) => {
                                        if i == metadata.modified_fields.len() - 1 {
                                            *table_state.selected_mut() = Some(0)
                                        } else {
                                            *table_state.selected_mut() = Some(i + 1)
                                        }
                                    }
                                    None => *table_state.selected_mut() = Some(0),
                                },
                                KeyCode::Up => match table_state.selected() {
                                    Some(i) => {
                                        if i == 0 {
                                            *table_state.selected_mut() =
                                                Some(metadata.modified_fields.len() - 1)
                                        } else {
                                            *table_state.selected_mut() = Some(i - 1)
                                        }
                                    }
                                    None => {
                                        *table_state.selected_mut() =
                                            Some(metadata.modified_fields.len() - 1)
                                    }
                                },
                                _ => {}
                            }
                        }
                    }
                }

                if !metadata.has_gps {
                    metadata.rotate_globe();
                }
            }
            restore_terminal()
        }
    }
}

fn view(metadata: &mut Application, frame: &mut Frame, table_state: &mut TableState) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            // Constraint::Percentage(15),
            Constraint::Percentage(40),
            Constraint::Percentage(60),
        ])
        .split(frame.size());
    //     frame.render_widget(
    //         Paragraph::new(
    //             r"____  ____  _____ ____ ____   ___  _   _
    // | __ )|  _ \| ____/ ___/ ___| / _ \| \ | |
    // |  _ \| |_) |  _| \___ \___ \| | | |  \| |
    // | |_) |  _ <| |___ ___) |__) | |_| | |\  |
    // |____/|_| \_\_____|____/____/ \___/|_| \_|",
    //         )
    //         .alignment(Alignment::Center)
    //         .block(Block::new().borders(Borders::ALL))
    //         .bold(),
    //         layout[0],
    //     );
    // let area = frame.size();
    let widths = [Constraint::Length(30), Constraint::Length(70)];
    let exif_table = Table::new(metadata.process_rows(), widths).column_spacing(1);
    frame.render_stateful_widget(
        exif_table
            .block(
                Block::new()
                    .title("Metadata")
                    .title_style(Style::new().bold())
                    .border_set(symbols::border::PLAIN)
                    .borders(Borders::TOP | Borders::RIGHT | Borders::LEFT),
            )
            .header(Row::new(vec!["Tag", "Data"]).bold())
            // .style(Style::new().bold())
            .highlight_style(
                Style::new()
                    .cyan()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray),
            )
            .highlight_symbol("> "),
        layout[0],
        table_state,
    );

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
                    .title("Image Location")
                    .title_style(Style::new().bold())
                    .border_set(collapsed_top_border_set)
                    .borders(Borders::ALL),
            )
            .x_bounds([0., 100.])
            .y_bounds([0., 50.])
            .paint(|ctx| {
                ctx.layer();
                let mut globe_canvas = globe::Canvas::new(75, 50, Some((1, 1)));
                globe_canvas.clear();
                metadata.globe.render_sphere(&mut globe_canvas);
                let (size_x, size_y) = globe_canvas.get_size();
                // default character size is 4 by 8
                for i in 0..size_y {
                    for j in 0..size_x {
                        let translated_i = 50 - i;
                        match globe_canvas.matrix[i][j] {
                            ' ' => ctx.print(j as f64, translated_i as f64, " "),

                            x => {
                                // Only useful when there is no z-axis panning going on
                                let long_lat_color = if metadata.has_gps
                                    && i == (size_y / 2) - 1
                                    && j == (size_x / 2) - 1
                                {
                                    x.to_string().red().bold().slow_blink()
                                } else {
                                    x.to_string().into()
                                };
                                ctx.print(j as f64, translated_i as f64, long_lat_color)
                                // ctx.print(j as f64, translated_i as f64, x.to_string())
                            }
                        }
                    }
                }
            }),
        layout[1],
    )
}

mod tui {
    use anyhow::Result;
    use crossterm::{
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    };
    use ratatui::{
        backend::Backend,
        prelude::{CrosstermBackend, Terminal},
    };
    use std::{io::stdout, panic};

    // Have the terminal be generic over a backend
    pub fn init_terminal() -> Result<Terminal<impl Backend>> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        Ok(terminal)
    }

    pub fn install_panic_hook() {
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            stdout().execute(LeaveAlternateScreen).unwrap();
            disable_raw_mode().unwrap();
            original_hook(panic_info);
        }));
    }

    pub fn restore_terminal() -> Result<()> {
        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;

        Ok(())
    }
}

// Orients the camera so that it focuses on the given target coordinates.
// fn focus_target(coords: (f32, f32), xy_offset: f32, cam_xy: &mut f32, cam_z: &mut f32) {
//     let (cx, cy) = coords;
//     *cam_xy = (cx * PI) * -1. - 1.5 - xy_offset;
//     *cam_z = cy * 3. - 1.5;
// }
