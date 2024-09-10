# Bresson

Bresson is an EXIF Metadata Processing TUI (Terminal User Interface) built in Rust using the [Ratatui](https://github.com/ratatui/ratatui) immediate-mode TUI library.

Bresson was initially conceived as an exploration of terminal interfaces and immediate-mode rendering during my Winter 2' 2024 Batch at the [Recurse Center](https://recurse.com).

![bresson](https://github.com/user-attachments/assets/80313921-dfd3-4c3c-b1f5-3ed8fddb1955)

## What does it do?

Bresson allows you to inspect & modify the Exif metadata for a given image inside your terminal.

Within TUI Mode, all of the metadata is displayed inside a table and with the appropriate units for relevant fields.

If the provided image has any GPS data, an ASCII rendering of Earth will be shown with the GPS co-ordinates highlighted!

There is also support for rendering the image thumbnail via [ratatui-image](https://github.com/benjajaja/ratatui-image) but this is highly dependent on what image backends are supported by your terminal. Recommended terminals are -

* XTerm
* Foot
* kitty
* Wezterm
* iTerm2
* Ghostty

## Usage

Here is a list of Keybinds for TUI mode -

| Keybind        | Description                                         |
| -------------- | --------------------------------------------------- |
| `r`            | Randomize the highlighted field                     |
| `R`            | Randomize all fields                                |
| `c`            | Clear selected metadata                             |
| `C`            | Clear all metadata                                  |
| `u`            | Undo change                                         |
| `U`            | Undo all changes / Restore                          |
| `t` \| `T`     | Toggle between displaying Thumbnail and Globe       |
| `s` \| `S`     | Save a copy of the modified metadata                |
| `g` \| `G`     | Toggle Globe Visibility                             |
| `<Spc>`        | Toggle Globe Rotation                               |
| `?`            | Show/Dismiss Keybind Info                           |
| `q` \| `<Esc>` | Exit the app                                        |


### Metadata that can be randomized

- `Make`
- `Model`
- `DateTime`
- `DateTimeOriginal`
- `DateTimeDigitized`
- `ExposureTime`
- `FNumber`
- `PhotographicSensitivity`
- `MeteringMode`
- `ColorSpace`
- `GPSLatitude`
- `GPSLongitude`
- `GPSLatitudeRef`
- `GPSLongitudeRef`

## Running Bresson

Currently Bresson is in alpha development. To build Bresson, please clone the repository to your local environment and then running the following command -

```shell
$ cargo run -- <PATH_TO_IMAGE>
```

This will build Bresson locally and then run it (in debug mode).

## Future Features

- [ ] Randomizing more metadata fields
- [ ] Editing metadata directly
- [x] Displaying the Thumbnail
- [ ] Configuration File
- [ ] Alternate Stylesheets
- [ ] DSL for defining modifications
- [ ] Batch processing a directory containing multiple images
- [ ] File Picker interface
