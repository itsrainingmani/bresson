use anyhow::Result;
use bresson::*;
use std::path::Path;

use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::{CrosstermBackend, Stylize, Terminal},
    style::{Color, Style},
    widgets::{
        canvas::{Canvas, Line, Map, MapResolution, Rectangle},
        Block, Borders, Paragraph, Row, Table,
    },
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

    let metadata = get_all_metadata(image_file)?;

    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    loop {
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
            let exif_table = Table::new(metadata.clone(), widths);
            frame.render_widget(
                exif_table
                    .block(Block::new().borders(Borders::ALL))
                    .header(Row::new(vec!["Tag", "Data"]))
                    .highlight_style(Style::new().light_cyan()),
                layout[1],
            );
            frame.render_widget(
                Canvas::default()
                    .block(Block::default().title("Canvas").borders(Borders::ALL))
                    .x_bounds([-180.0, 180.0])
                    .y_bounds([-90.0, 90.0])
                    .paint(|ctx| {
                        ctx.draw(&Map {
                            resolution: MapResolution::High,
                            color: Color::White,
                        });
                        ctx.layer();
                        ctx.draw(&Line {
                            x1: 0.0,
                            y1: 10.0,
                            x2: 10.0,
                            y2: 10.0,
                            color: Color::White,
                        });
                        ctx.draw(&Rectangle {
                            x: 10.0,
                            y: 20.0,
                            width: 10.0,
                            height: 10.0,
                            color: Color::Red,
                        });
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
