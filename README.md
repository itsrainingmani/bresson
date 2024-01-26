# Bresson

EXIF Metadata Processing Tool in Rust

## CLI Args

Defaults -

- Do not mutate the original file
- Always write to a copy unless specified with the `mutate` flag

<!-- - `-c` | `--cli` to run in command line mode -->

- `-r` | `--random` to automatically randomize the exif metadata
- (Shouldn't this be the default?)
- `-m` | `--mutate` to mutate the original file (not recommended)

## TUI Mode

The TUI Mode allows you to inspect & modify the metadata for a given image using RataTUI.

Within TUI Mode, all of the metadata is displayed inside a table and with the appropriate units for relevant fields.

Here is a list of Keybinds for TUI mode -

- `c` | `C` to clear all metadata
- `r` to randomize the highlighted field
- `R` to randomize all fields (all here refers to fields that can be randomized)
- `s` | `S` to save a copy of the modified metadata
- `o` | `O` to display the original metadata for the image

## Metadata that can be Randomzied

Make
Model
DateTime
XResolution
YResolution
Software
ModifyDate
Artist
Copyright
ExposureTime
FNumber
FocalLength
ISO
MeteringMode
GPSLatitude
GPSLongitude
GPSLatitudeRef
GPSLongitudeRef
