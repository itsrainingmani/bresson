use anyhow::Result;
use exif::{Exif, Field, In, Tag, Value};
use globe::Globe;
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
    pub exif_data: ExifTags,
    pub exif: Exif,
    pub globe: Globe,
    pub app_mode: ApplicationMode,
    pub has_gps: bool,
    pub gps_info: GPSInfo,
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
                    exif_data_rows.push(f.clone());
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
            exif_data: exif_data_rows,
            exif,
            globe: g,
            app_mode,
            has_gps,
            gps_info,
        })
    }

    pub fn process_rows(&self) -> Vec<Row> {
        let mut exif_data_rows = Vec::new();
        for f in &self.exif_data {
            let f_val = f.tag.to_string();
            if f_val.len() > 0 {
                exif_data_rows.push(Row::new(vec![
                    f.tag.to_string(),
                    f.display_value().with_unit(&self.exif).to_string(),
                ]));
            }
        }

        exif_data_rows
    }

    pub fn update_globe_rotation(&mut self) {
        self.globe.angle += 0.01;
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

        self.globe.camera.update(1.5, new_longitude, new_latitude);
    }

    pub fn randomize(&self) -> ExifTags {
        let mut random_data: ExifTags = Vec::new();

        for f in &self.exif_data {
            match f.tag {
                Tag::Make
                | Tag::Model
                | Tag::DateTime
                | Tag::XResolution
                | Tag::YResolution
                | Tag::Software
                | Tag::DateTimeOriginal
                | Tag::Artist
                | Tag::Copyright
                | Tag::ExposureTime
                | Tag::FNumber
                | Tag::FocalLength
                | Tag::ISOSpeed
                | Tag::MeteringMode
                | Tag::GPSLatitude
                | Tag::GPSLongitude
                | Tag::GPSLatitudeRef
                | Tag::GPSLongitudeRef => {
                    random_data.push(f.clone());
                }
                _ => {}
            }
        }

        random_data
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
