use anyhow::Result;
use bresson::globe::Globe;
use bresson::*;
use std::{f32::consts::PI, path::Path};

use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::{CrosstermBackend, Stylize, Terminal},
    style::Style,
    widgets::{canvas::*, Block, Borders, Paragraph, Row, Table},
};
use std::io::stdout;

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
    let mut globe = Globe::new(1., 0., false);
    globe.camera.update(cam_zoom, cam_xy, cam_z);
    // globe.angle += 1.1;
    // let metadata = get_all_metadata(image_file)?;
    let mut metadata = ExifMetadata::new(image_file, globe)?;

    // cam_xy -= 1.0 / 2000.;
    // globe.camera.update(cam_zoom, cam_xy, cam_z);

    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    // let globe_rot_speed = 1. / 1000.;

    loop {
        metadata.update_globe_rotation();
        terminal.draw(|frame| {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Percentage(10),
                    Constraint::Percentage(30),
                    Constraint::Percentage(60),
                ])
                .split(frame.size());
            frame.render_widget(
                Paragraph::new("BRESSON")
                    .block(Block::new().borders(Borders::ALL))
                    .bold(),
                layout[0],
            );
            // let area = frame.size();
            let widths = [Constraint::Length(30), Constraint::Length(30)];
            let exif_table = Table::new(metadata.process_rows(), widths);
            frame.render_widget(
                exif_table
                    .block(Block::new().borders(Borders::ALL))
                    .header(Row::new(vec!["Tag", "Data"]))
                    .highlight_style(Style::new().light_cyan()),
                layout[1],
            );
            frame.render_widget(
                Canvas::default()
                    .block(Block::default().title("Map").borders(Borders::ALL))
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
                                    x => ctx.print(j as f64, translated_i as f64, x.to_string()),
                                }
                            }
                        }
                    }),
                layout[2],
            )
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}

/// Orients the camera so that it focuses on the given target coordinates.
fn focus_target(coords: (f32, f32), xy_offset: f32, cam_xy: &mut f32, cam_z: &mut f32) {
    let (cx, cy) = coords;
    *cam_xy = (cx * PI) * -1. - 1.5 - xy_offset;
    *cam_z = cy * 3. - 1.5;
}
