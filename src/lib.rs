pub mod globe;

use anyhow::Result;
use exif::{Exif, Field, In, Tag, Value};
use globe::Globe;
use rand::seq::SliceRandom;
use ratatui::widgets::Row;
use std::path::{Path, PathBuf};
// Step one is taking a given image file and read out some of the super basic metadata about it

#[derive(Debug, Clone, Copy)]
pub enum ApplicationMode {
    CommandLine,
    Interactive,
}

#[derive(Debug, Clone, Copy)]
pub enum Cardinal {
    North,
    East,
    West,
    South,
}

pub struct GPSInfo {
    latitude: f32,
    lat_direction: Cardinal,
    longitude: f32,
    long_direction: Cardinal,
}

pub struct CameraSettings {
    zoom: f32,
    alpha: f32, // Rotation along xy-axis
    beta: f32,  // Rotation along z-axis
}

impl Default for GPSInfo {
    fn default() -> Self {
        Self {
            latitude: Default::default(),
            lat_direction: Cardinal::North,
            longitude: Default::default(),
            long_direction: Cardinal::East,
        }
    }
}

pub type ExifTags = Vec<Field>;

pub struct Model {
    pub path_to_image: PathBuf,
    pub exif: Exif,
    pub original_fields: ExifTags,
    pub randomized_fields: ExifTags,
    pub globe: Globe,
    pub app_mode: ApplicationMode,
    pub has_gps: bool,
    pub gps_info: GPSInfo,
    pub camera_settings: CameraSettings,
}

impl Model {
    pub fn new(path_to_image: &Path, g: Globe, app_mode: ApplicationMode) -> Result<Self> {
        let file = std::fs::File::open(path_to_image)?;
        let mut bufreader = std::io::BufReader::new(&file);
        let exifreader = exif::Reader::new();
        let exif = exifreader.read_from_container(&mut bufreader)?;
        let mut has_gps = false;

        let mut exif_data_rows: ExifTags = Vec::new();
        for f in exif.fields() {
            match f.tag {
                Tag::Make
                | Tag::Model
                // | Tag::DateTime
                // | Tag::XResolution
                // | Tag::YResolution
                | Tag::Software
                | Tag::DateTimeOriginal
                // | Tag::Artist
                // | Tag::Copyright
                | Tag::ExposureTime
                | Tag::FNumber
                // | Tag::FocalLength
                | Tag::ISOSpeed
                | Tag::MeteringMode => {
                    exif_data_rows.push(f.clone());
                }
                Tag::GPSLatitude | Tag::GPSLongitude => {
                    has_gps = true;
                }
                _ => {}
            }
        }

        let gps_info = if has_gps {
            let lat: f32 = match exif.get_field(Tag::GPSLatitude, In::PRIMARY) {
                Some(l) => match l.value {
                    Value::Rational(ref v) if !v.is_empty() => {
                        (v[0].num as f32 / v[0].denom as f32)
                            + (v[1].num as f32 / v[1].denom as f32) / 60.
                            + (v[2].num as f32 / v[2].denom as f32) / (60. * 100.)
                    }
                    _ => 0.,
                },
                None => 0.,
            };
            let long: f32 = match exif.get_field(Tag::GPSLongitude, In::PRIMARY) {
                Some(l) => match l.value {
                    Value::Rational(ref v) if !v.is_empty() => {
                        (v[0].num as f32 / v[0].denom as f32)
                            + (v[1].num as f32 / v[1].denom as f32) / 60.
                            + (v[2].num as f32 / v[2].denom as f32) / (60. * 100.)
                    }
                    _ => 0.,
                },
                None => 0.,
            };
            let lat_dir = match exif.get_field(Tag::GPSLatitudeRef, In::PRIMARY) {
                Some(l) => {
                    let display_value = &l.display_value().to_string();
                    let str_val = display_value.as_str();
                    match str_val {
                        "N" => Cardinal::North,
                        "S" => Cardinal::South,
                        _ => Cardinal::North,
                    }
                }
                None => Cardinal::North,
            };
            let long_dir = match exif.get_field(Tag::GPSLongitudeRef, In::PRIMARY) {
                Some(l) => {
                    let display_value = &l.display_value().to_string();
                    let str_val = display_value.as_str();
                    match str_val {
                        "E" => Cardinal::East,
                        "W" => Cardinal::West,
                        _ => Cardinal::North,
                    }
                }
                None => Cardinal::East,
            };
            GPSInfo {
                latitude: lat,
                lat_direction: lat_dir,
                longitude: long,
                long_direction: long_dir,
            }
        } else {
            GPSInfo::default()
        };

        Ok(Self {
            path_to_image: path_to_image.to_path_buf(),
            exif,
            original_fields: exif_data_rows.clone(),
            randomized_fields: exif_data_rows.clone(),
            globe: g,
            app_mode,
            has_gps,
            gps_info,
            camera_settings: CameraSettings {
                zoom: 1.5,
                alpha: 0.,
                beta: 0.,
            },
        })
    }

    pub fn process_rows(&self) -> Vec<Row> {
        let mut exif_data_rows = Vec::new();
        for f in &self.randomized_fields {
            let f_val = f.tag.to_string();
            if f_val.len() > 0 {
                exif_data_rows.push(Row::new(vec![
                    f.tag.to_string(),
                    f.display_value()
                        .with_unit(&self.exif)
                        .to_string()
                        .trim_matches('"')
                        .to_string(),
                ]));
            }
        }

        exif_data_rows
    }

    pub fn update_globe_rotation(&mut self) {
        self.globe.angle += 0.01;
    }

    pub fn camera_zoom_increase(&mut self) {
        self.camera_settings.zoom -= 0.01;
        self.globe.camera.update(
            self.camera_settings.zoom,
            self.camera_settings.alpha,
            self.camera_settings.beta,
        );
    }

    pub fn camera_zoom_decrease(&mut self) {
        self.camera_settings.zoom += 0.01;
        self.globe.camera.update(
            self.camera_settings.zoom,
            self.camera_settings.alpha,
            self.camera_settings.beta,
        );
    }

    pub fn transform_coordinates(&mut self) {
        // Latitude is 0 at the equator and increases to 90N for the north pole
        // and 90S for the South Pole
        // Longitude is 0 at the Prime Meridian (Greenwich) and increases to 180E at the
        // 180th Meridian
        // Latitude is a -90 -> 90 spread
        // Longitude is a -180 -> 180 spread

        let new_longitude = match self.gps_info.long_direction {
            Cardinal::East => self.gps_info.longitude,
            Cardinal::West => 360. - self.gps_info.longitude, // Convert into Long East
            _ => 0.0,
        } / 360.;
        let new_latitude = match self.gps_info.lat_direction {
            Cardinal::North => self.gps_info.latitude / 90.,
            Cardinal::South => -self.gps_info.latitude / 90.,
            _ => 0.,
        };
        self.camera_settings = CameraSettings {
            zoom: 1.45,
            alpha: new_longitude,
            beta: new_latitude,
        };

        self.globe.camera.update(1.45, new_longitude, new_latitude);
    }

    pub fn randomize(&mut self) {
        let mut rng = rand::thread_rng();
        let mut random_data: ExifTags = Vec::new();
        let camera_manufacturers = vec![
            "Canon",
            "Nikon",
            "Sony",
            "Fujifilm",
            "Panasonic",
            "Olympus",
            "Leica",
            "Pentax",
            "Samsung",
            "GoPro",
            "Hasselblad",
            "DJI",
            "Phase One",
            "Ricoh",
            "Sigma",
            "Hoya",
            "Kodak",
            "YI Technology",
            "Lytro",
            "RED Digital Cinema",
        ];

        for f in &self.randomized_fields {
            match f.tag {
                Tag::Make => {
                    let rand_make = *camera_manufacturers.choose(&mut rng).unwrap();
                    random_data.push(Field { tag: Tag(exif::Context::Tiff, 271), ifd_num: In(0), value: Value::Ascii(vec![Vec::from(rand_make)]) });
                }
                Tag::Model
                // | Tag::DateTime
                // | Tag::XResolution
                // | Tag::YResolution
                | Tag::Software
                | Tag::DateTimeOriginal
                // | Tag::Artist
                // | Tag::Copyright
                | Tag::ExposureTime
                | Tag::FNumber
                // | Tag::FocalLength
                | Tag::ISOSpeed
                | Tag::MeteringMode => {
                    random_data.push(f.clone());
                }
                _ => {}
            }
        }

        self.randomized_fields = random_data;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn check_coordinate_transform() {
    //     println!(
    //         "{:?}",
    //         Model::transform_coordinates(40.7128, Cardinal::North, 74.0060, Cardinal::West)
    //     );
    // }
}
