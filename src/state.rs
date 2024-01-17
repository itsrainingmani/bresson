use crate::globe::Globe;
use anyhow::Result;
use chrono::prelude::*;
<<<<<<< HEAD
use exif::{Exif, Field, In, Rational, Tag, Value};
=======
use exif::{experimental::Writer, Exif, Field, In, Rational, Tag, Value};
use globe::Globe;
>>>>>>> af6534d (exploring clearing metadata)
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use ratatui::widgets::Row;
use std::{
    collections::HashSet,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
// Step one is taking a given image file and read out some of the super basic metadata about it

#[derive(Debug, Clone, Copy)]
pub enum AppMode {
    CommandLine,
    Interactive,
}

#[derive(Debug, Clone, Copy)]
pub enum RenderState {
    Normal,
    Help,
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

pub struct Application {
    pub path_to_image: PathBuf,
    pub exif: Exif,
    pub original_fields: ExifTags,
    pub randomized_fields: ExifTags,
    pub tags_to_randomize: HashSet<Tag>,
    pub globe: Globe,
    pub app_mode: AppMode,
    pub has_gps: bool,
    pub gps_info: GPSInfo,
    pub camera_settings: CameraSettings,
}

pub fn random_datetime(rng: &mut ThreadRng) -> String {
    let now_utc = Utc::now();
    let date_utc = now_utc.date_naive();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        rng.gen_range(2001..=date_utc.year_ce().1),
        rng.gen_range(1..=(date_utc.month0() + 1)),
        rng.gen_range(1..=(date_utc.day0() + 1)),
        rng.gen_range(0..=now_utc.hour()),
        rng.gen_range(0..=now_utc.minute()),
        rng.gen_range(0..=now_utc.second())
    )
}

impl Application {
    pub fn new(path_to_image: &Path, g: Globe, app_mode: AppMode) -> Result<Self> {
        let file = std::fs::File::open(path_to_image)?;

        // println!("Size of img is {}", file.metadata()?.len());

        let mut bufreader = std::io::BufReader::new(&file);
        let exifreader = exif::Reader::new();
        let exif = exifreader.read_from_container(&mut bufreader)?;
        let mut has_gps = false;

        let tags_to_randomize = HashSet::from([
            Tag::Make,
            Tag::Model,
            Tag::DateTimeOriginal,
            Tag::ExposureTime,
            Tag::FNumber,
            Tag::MeteringMode,
            Tag::ColorSpace,
        ]);

        let mut exif_data_rows: ExifTags = Vec::new();
        for f in exif.fields() {
            match f.tag {
                // Tag::Make
                // | Tag::Model
                // | Tag::Software
                // | Tag::DateTimeOriginal
                // | Tag::CameraOwnerName
                // | Tag::ExposureTime
                // | Tag::FNumber
                // | Tag::FocalLength
                // | Tag::ISOSpeed
                // | Tag::Humidity
                // | Tag::CameraElevationAngle
                // | Tag::Pressure
                // | Tag::Compression
                // | Tag::Contrast
                // // | Tag::Orientation
                // | Tag::ColorSpace
                // | Tag::MeteringMode => {
                //     exif_data_rows.push(f.clone());
                // }
                Tag::GPSLatitude | Tag::GPSLongitude => {
                    has_gps = true;
                }
                _ => {
                    exif_data_rows.push(f.clone());
                }
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
            tags_to_randomize,
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

    pub fn rotate_globe(&mut self) {
        let globe_rot_speed = 1. / 1000.;
        let cam_rot_speed = 1. / 1000.;
        self.globe.angle += globe_rot_speed;
        self.camera_settings.alpha += cam_rot_speed + (globe_rot_speed / 2.);

        self.globe.camera.update(
            self.camera_settings.zoom,
            self.camera_settings.alpha,
            self.camera_settings.beta,
        );
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

    pub fn randomize_all(&mut self) {
        for i in 0..self.randomized_fields.len() {
            self.randomize(i);
        }
    }

    pub fn randomize(&mut self, index: usize) {
        let mut rng = rand::thread_rng();
        // let mut random_data: ExifTags = Vec::new();
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
        let f_numbers = vec![1, 2, 3, 4, 5, 8, 11, 16, 22, 32, 45, 64];

        match self.randomized_fields.get_mut(index) {
            Some(f) => {
                // println!("{:?}", f);
                match f.tag {
                    Tag::Make => {
                        f.value = Value::Ascii(vec![Vec::from(
                            *camera_manufacturers.choose(&mut rng).unwrap(),
                        )]);
                    }
                    Tag::ExposureTime => {
                        f.value = Value::Rational(vec![Rational {
                            num: 1,
                            denom: rand::random::<u8>() as u32,
                        }]);
                    }
                    Tag::FNumber => {
                        f.value = Value::Rational(vec![Rational {
                            num: *f_numbers.choose(&mut rng).unwrap(),
                            denom: rng.gen_range(1..=3),
                        }]);
                    }
                    Tag::MeteringMode => f.value = Value::Short(vec![rng.gen_range(1..=6)]),
                    Tag::DateTimeOriginal => {
                        f.value = Value::Ascii(vec![Vec::from(random_datetime(&mut rng))]);
                    }
                    _ => {}
                }
            }
            None => {}
        }

        // self.randomized_fields = random_data;
    }

    pub fn clear_exif_data(&mut self) -> Result<()> {
        // Zero out all available tags
        // todo!()
        // Internals of Exif read_from_container
        // reader.by_ref().take(4096).read_to_end(&mut buf)?;
        // take -> creates an adapter which will read at most "limit" bytes from it
        let exif_buf = self.exif.buf();
        let size_of_exif_buf = exif_buf.len();
        println!("Size of og exif buf: {}", size_of_exif_buf);

        let mut buf = Vec::new();
        let mut handle = exif_buf.take(20);
        _ = handle.read_to_end(&mut buf)?;
        for b in buf.bytes() {
            println!("{:#x}", b.unwrap());
        }

        let exif_ver = Field {
            tag: Tag::ExifVersion,
            ifd_num: In::PRIMARY,
            value: Value::Undefined(b"0231".to_vec(), 0),
        };

        let mut exif_writer = Writer::new();
        let mut new_exif_buf = io::Cursor::new(Vec::new());
        exif_writer.push_field(&exif_ver);
        exif_writer.write(&mut new_exif_buf, false)?;
        let mut new_exif_buf = new_exif_buf.clone().into_inner();
        println!("Size of new exif buf: {}", new_exif_buf.len());
        for b in new_exif_buf.bytes() {
            println!("{:#x}", b.unwrap());
        }

        let file = std::fs::File::open(&self.path_to_image)?;
        let mut bufreader = std::io::BufReader::new(&file);
        let mut img_buf = Vec::new();

        _ = bufreader.read_to_end(&mut img_buf);

        let img_data = &img_buf[size_of_exif_buf - 1..];
        new_exif_buf.extend_from_slice(&img_data);
        println!("{}", new_exif_buf.len());

        let mut copy_file_path = self.path_to_image.clone();
        let copy_file_name = copy_file_path.file_name().expect("Valid File Name");

        copy_file_path.set_file_name(format!("copy-{}", copy_file_name.to_str().unwrap()));
        println!("{}", copy_file_path.display());

        let mut copy_file = std::fs::File::create(copy_file_path)?;
        copy_file.write_all(new_exif_buf.as_slice())?;

        Ok(())
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
