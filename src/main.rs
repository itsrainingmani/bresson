use anyhow::Result;
use bresson::*;
use globe::{CameraConfig, GlobeConfig, GlobeTemplate};
use std::{f32::consts::PI, path::Path};
use tui::restore_terminal;

use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::Stylize,
    style::{Modifier, Style},
    widgets::{canvas::*, Block, Borders, Paragraph, Row, Table},
    Frame,
};

fn main() -> Result<()> {
    let args = std::env::args();
    if args.len() < 2 {
        std::process::exit(1);
    }
    let image_arg = std::env::args().nth(1).unwrap();
    let image_file = Path::new(&image_arg);
    if image_file.is_file() {
        println!("Image: {}", image_file.display());
    } else {
        println!("Image not present");
        return Ok(());
    }
    let cam_zoom = 1.5;
    let cam_xy = 0.;
    let cam_z = 0.;
    // let init_coords = (51.32, 51.32);
    // focus_target(init_coords, 0., &mut cam_xy, &mut cam_z);
    // cam_z += 0.2;
    let globe = GlobeConfig::new()
        .use_template(GlobeTemplate::Earth)
        .with_camera(CameraConfig::new(cam_zoom, cam_xy, cam_z))
        // .display_night(true)
        .build();
    let mut metadata = ExifMetadata::new(image_file, globe)?;

    tui::install_panic_hook();
    let mut terminal = tui::init_terminal()?;
    terminal.clear()?;

    loop {
        terminal.draw(|frame| view(&mut metadata, frame))?;
        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

        // Update the Globe Rotation
        metadata.update_globe_rotation();
    }

    restore_terminal()
}

fn view(metadata: &mut ExifMetadata, frame: &mut Frame) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Percentage(15),
            Constraint::Percentage(25),
            Constraint::Percentage(60),
        ])
        .split(frame.size());
    frame.render_widget(
        Paragraph::new(
            r" ____  ____  _____ ____ ____   ___  _   _ 
| __ )|  _ \| ____/ ___/ ___| / _ \| \ | |
|  _ \| |_) |  _| \___ \___ \| | | |  \| |
| |_) |  _ <| |___ ___) |__) | |_| | |\  |
|____/|_| \_\_____|____/____/ \___/|_| \_|",
        )
        .alignment(Alignment::Center)
        .block(Block::new().borders(Borders::ALL))
        .bold(),
        layout[0],
    );
    // let area = frame.size();
    let widths = [Constraint::Length(30), Constraint::Length(30)];
    let exif_table = Table::new(metadata.process_rows(), widths).column_spacing(1);
    frame.render_widget(
        exif_table
            .block(
                Block::new()
                    .title("Metadata")
                    .title_style(Style::new().bold())
                    .borders(Borders::ALL),
            )
            .header(Row::new(vec!["Tag", "Data"]).bold())
            // .style(Style::new().bold())
            .highlight_style(Style::new().light_cyan().add_modifier(Modifier::BOLD))
            .highlight_symbol(">>"),
        layout[1],
    );
    frame.render_widget(
        Canvas::default()
            .block(
                Block::default()
                    .title("Map")
                    .title_style(Style::new().bold())
                    .borders(Borders::ALL),
            )
            .x_bounds([0., 100.])
            .y_bounds([0., 50.])
            .paint(|ctx| {
                ctx.layer();
                let mut globe_canvas = globe::Canvas::new(75, 50, Some((1, 1)));
                globe_canvas.clear();
                metadata.globe.render_on(&mut globe_canvas);
                let (size_x, size_y) = globe_canvas.get_size();
                // default character size is 4 by 8
                for i in 0..size_y {
                    for j in 0..size_x {
                        let translated_i = 50 - i;
                        match globe_canvas.matrix[i][j] {
                            ' ' => ctx.print(j as f64, translated_i as f64, " "),
                            x => ctx.print(j as f64, translated_i as f64, x.to_string()),
                        }
                    }
                }
            }),
        layout[2],
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

/// Orients the camera so that it focuses on the given target coordinates.
fn focus_target(coords: (f32, f32), xy_offset: f32, cam_xy: &mut f32, cam_z: &mut f32) {
    let (cx, cy) = coords;
    *cam_xy = (cx * PI) * -1. - 1.5 - xy_offset;
    *cam_z = cy * 3. - 1.5;
}
