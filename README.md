# Bresson

EXIF Metadata Processing TUI (Terminal User Interface) in Rust

## TUI Mode

The TUI Mode allows you to inspect & modify the metadata for a given image inside your terminal.

Within TUI Mode, all of the metadata is displayed inside a table and with the appropriate units for relevant fields.

Here is a list of Keybinds for TUI mode -

| Keybind        | Description                                         |
| -------------- | --------------------------------------------------- |
| `r`            | Randomize the highlighted field                     |
| `R`            | Randomize all fields                                |
| `c` \| `C`     | Clear all metadata                                  |
| `o` \| `O`     | Display the original metadata for the image         |
| `t` \| `T`     | Toggle between displaying Image Thumbnail and Globe |
| `s` \| `S`     | Save a copy of the modified metadata                |
| `g` \| `G`     | Toggle Globe Visibility                             |
| `<Spc>`        | Toggle Globe Rotation                               |
| `q` \| `<Esc>` | Exit the app                                        |
| `?`            | Show/Dismiss Keybind Info                           |

### Metadata that can be randomized

- Make
- Model
- DateTime
- DateTimeOriginal
- DateTimeDigitized
- ExposureTime
- FNumber
- MeteringMode
- GPSLatitude
- GPSLongitude
- GPSLatitudeRef
- GPSLongitudeRef

## CLI Args

Defaults -

- Do not mutate the original file
- Always write to a copy unless specified with the `mutate` flag

<!-- - `-c` | `--cli` to run in command line mode -->

- `-r` | `--random` to automatically randomize the exif metadata
- (Shouldn't this be the default?)
- `-m` | `--mutate` to mutate the original file (not recommended)

## Future Features

- [ ] Randomizing more metadata fields
- [ ] Editing metadata directly
- [ ] Displaying the Thumbnail
- [ ] Configuration File
- [ ] Alternate Stylesheets
- [ ] DSL for defining modifications
- [ ] Batch processing a directory containing multiple images
- [ ] File Picker interface
