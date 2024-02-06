use anyhow::Result;
use bresson::globe::Globe;
use bresson::{state::*, ui::*};
use ratatui_image::{protocol::StatefulProtocol, Resize};
use std::{path::Path, sync::mpsc, thread, time::Duration};
use tui::restore_terminal;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::TableState};
// use ratatui_image::picker::Picker;

enum AppEvent {
    KeyEvent(KeyEvent),
    Redraw(Box<dyn StatefulProtocol>),
}

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
    if !image_file.is_file() {
        eprintln!("Image not present");
        return Ok(());
    }
    let cam_zoom = 1.5;
    let cam_xy = 0.;
    let cam_z = 0.;
    let mut globe = Globe::new(1., 0., false);
    globe.camera.update(cam_zoom, cam_xy, cam_z);

    // Send a [ResizeProtocol] to resize and encode it in a separate thread.
    let (tx_worker, rec_worker) = mpsc::channel::<(Box<dyn StatefulProtocol>, Resize, Rect)>();

    // Send UI-events and the [ResizeProtocol] result back to main thread.
    let (tx_main, rec_main) = mpsc::channel();

    // Resize and encode in background thread.
    let tx_main_render = tx_main.clone();
    thread::spawn(move || loop {
        if let Ok((mut protocol, resize, area)) = rec_worker.recv() {
            protocol.resize_encode(&resize, None, area);
            tx_main_render.send(AppEvent::Redraw(protocol)).unwrap();
        }
    });
    let mut app = Application::new(image_file, globe, app_mode, tx_worker)?;

    // Poll events in background thread to demonstrate polling terminal events and redraw events
    // concurrently. It's not required to do it this way - the "redraw event" from the channel
    // could be read after polling the terminal events (as long as it's done with a timout). But
    // then the rendering of the image will always be somewhat delayed.
    let tx_main_events = tx_main.clone();
    thread::spawn(move || -> Result<(), std::io::Error> {
        loop {
            if crossterm::event::poll(Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    tx_main_events.send(AppEvent::KeyEvent(key)).unwrap();
                }
            }
        }
    });
    let mut table_state = TableState::new().with_selected(Some(0));
    match app.app_mode {
        AppMode::CommandLine => {
            // Print out the Exif Data in the CLI
            app.clear_fields();
            app.save_state()?;
            Ok(())
        }
        AppMode::Interactive => {
            tui::install_panic_hook();
            let mut terminal = tui::init_terminal()?;
            terminal.clear()?;

            if app.has_gps {
                // Go to the coordinates extracted from the input
                app.transform_coordinates();
            }

            app.show_message(format!("Opened {:?}", app.path_to_image.clone()));

            loop {
                terminal.draw(|frame| view(&mut app, frame, &mut table_state))?;
                if let Ok(ev) = rec_main.try_recv() {
                    match ev {
                        AppEvent::KeyEvent(key) => {
                            if key.kind == KeyEventKind::Press && !app.show_keybinds {
                                match key.code {
                                    KeyCode::Char(c) => match c {
                                        'o' | 'O' => {
                                            // Show Original Data
                                            app.modified_fields = app.original_fields.clone();
                                            if app.has_gps && !app.should_rotate {
                                                app.transform_coordinates();
                                            }
                                            app.show_message("Showing Original Data".to_owned());
                                        }
                                        'r' => {
                                            // Only randomize the selected element based on table state
                                            match table_state.selected() {
                                                Some(index) => {
                                                    app.randomize(index);
                                                }
                                                None => {}
                                            }
                                        }
                                        'R' => {
                                            // Randomize all fields (generalize over the individual field)
                                            app.randomize_all();
                                            app.show_message("Randomized all".to_owned());
                                        }
                                        'c' | 'C' => {
                                            app.clear_fields();
                                            app.show_message("Cleared Metadata".to_owned())
                                        }
                                        's' | 'S' => {
                                            // Save the state into a file copy
                                            app.save_state()?;
                                        }
                                        '?' => {
                                            // Display a popup window with keybinds
                                            // toggle the show_keybinds state
                                            app.toggle_keybinds();
                                        }
                                        'q' => break,
                                        '+' => app.camera_zoom_increase(),
                                        '-' => app.camera_zoom_decrease(),
                                        ' ' => app.toggle_rotate(),
                                        _ => {}
                                    },
                                    KeyCode::Esc => {
                                        break;
                                    }
                                    KeyCode::Down => match table_state.selected() {
                                        Some(i) => {
                                            if i == app.modified_fields.len() - 1 {
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
                                                    Some(app.modified_fields.len() - 1)
                                            } else {
                                                *table_state.selected_mut() = Some(i - 1)
                                            }
                                        }
                                        None => {
                                            *table_state.selected_mut() =
                                                Some(app.modified_fields.len() - 1)
                                        }
                                    },
                                    _ => {}
                                }
                            } else {
                                match key.code {
                                    KeyCode::Char(c) => match c {
                                        '?' => {
                                            // Display a popup window with keybinds
                                            // toggle the show_keybinds state
                                            app.toggle_keybinds();
                                        }
                                        _ => {}
                                    },
                                    KeyCode::Esc => {
                                        app.toggle_keybinds();
                                    }
                                    _ => {}
                                }
                            }
                        }
                        AppEvent::Redraw(protocol) => app.async_state.inner = Some(protocol),
                    }
                }

                if app.should_rotate {
                    app.rotate_globe();
                }
            }
            restore_terminal()
        }
    }
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
