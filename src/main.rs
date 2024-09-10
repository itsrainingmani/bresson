use bresson::{globe::Globe, state::*, tui, ui::*};
use ratatui_image::{protocol::StatefulProtocol, Resize};
use std::{path::Path, sync::mpsc, thread, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::TableState};

enum AppEvent {
    KeyEvent(KeyEvent),
    Redraw(Box<dyn StatefulProtocol>),
}

fn main() -> anyhow::Result<()> {
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
                AppMode::InteractiveFile
            }
        }
        None => AppMode::InteractiveFile,
    };

    let image_file = Path::new(&image_arg);
    if !image_file.is_file() {
        eprintln!("Image not present");
        return Ok(());
    }

    let cam_zoom = 1.5;
    let mut globe = Globe::new(1., 0., false);
    globe.camera.update(cam_zoom, 0., 0.);

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
            for f in app.exif.fields() {
                println!("Tag::{}", f.tag.to_string());
            }
            Ok(())
        }
        AppMode::InteractiveFile => {
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
                                        'u' | 'U' => {
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
                                        'c' => match table_state.selected() {
                                            Some(index) => {
                                                app.clear_field(index);
                                            }
                                            None => {}
                                        },
                                        'C' => {
                                            app.clear_all_fields();
                                            app.show_message("Cleared All Metadata".to_owned())
                                        }
                                        's' | 'S' => {
                                            // Save the state into a file copy
                                            app.save_state()?;
                                            app.show_message("Saved app state".to_owned());
                                        }
                                        'g' | 'G' => {
                                            app.toggle_globe();
                                            if app.show_globe {
                                                app.show_message("Showing Globe".to_owned());
                                            } else {
                                                app.should_rotate = false;
                                                app.show_message("Hiding Globe".to_owned());
                                            }
                                        }
                                        't' | 'T' => app.toggle_render_state(),
                                        '?' => {
                                            // Display a popup window with keybinds
                                            // toggle the show_keybinds state
                                            app.toggle_keybinds();
                                            if app.show_keybinds {
                                                app.show_message(
                                                    "Showing Keybinds window".to_owned(),
                                                );
                                            } else {
                                                app.show_message("Hid Keybinds window".to_owned());
                                            }
                                        }
                                        '+' => app.camera_zoom_increase(),
                                        '-' => app.camera_zoom_decrease(),
                                        ' ' => app.toggle_rotate(),
                                        'q' => break,
                                        _ => {}
                                    },
                                    KeyCode::Esc => {
                                        break;
                                    }
                                    KeyCode::Down | KeyCode::Tab => match table_state.selected() {
                                        Some(i) => {
                                            if i == app.modified_fields.len() - 1 {
                                                *table_state.selected_mut() = Some(0)
                                            } else {
                                                *table_state.selected_mut() = Some(i + 1)
                                            }
                                        }
                                        None => *table_state.selected_mut() = Some(0),
                                    },
                                    KeyCode::Up | KeyCode::BackTab => {
                                        match table_state.selected() {
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
                                        }
                                    }
                                    _ => {}
                                }
                            } else {
                                match key.code {
                                    KeyCode::Char(c) => match c {
                                        '?' => {
                                            // Display a popup window with keybinds
                                            // toggle the show_keybinds state
                                            app.toggle_keybinds();
                                            if app.show_keybinds {
                                                app.show_message(
                                                    "Showing Keybinds window".to_owned(),
                                                );
                                            } else {
                                                app.show_message("Hid Keybinds window".to_owned());
                                            }
                                        }
                                        _ => {}
                                    },
                                    KeyCode::Esc => {
                                        app.toggle_keybinds();
                                        if app.show_keybinds {
                                            app.show_message("Showing Keybinds window".to_owned());
                                        } else {
                                            app.show_message("Hid Keybinds window".to_owned());
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        AppEvent::Redraw(protocol) => app.async_state.set_protocol(protocol),
                    }
                }

                if app.should_rotate {
                    app.rotate_globe();
                }
            }
            tui::restore_terminal()
        }
    }
}
