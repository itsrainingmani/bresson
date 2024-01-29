use crate::utils::floyd_steinberg;
use anyhow::Result;
use chrono::prelude::*;
use core::f32;
use exif::{experimental::Writer, Exif, Field, In, Rational, SRational, Tag, Value};
use globe::Globe;
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use ratatui::widgets::Row;
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
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

#[derive(Debug)]
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
    pub image: Box<dyn StatefulProtocol>,
    pub exif: Exif,
    pub original_fields: ExifTags,
    pub modified_fields: ExifTags,
    pub tags_to_randomize: HashSet<Tag>,
    pub globe: Globe,
    pub app_mode: AppMode,
    pub has_gps: bool,
    pub gps_info: GPSInfo,
    pub camera_settings: CameraSettings,
    pub show_keybinds: bool,
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

        let mut picker = Picker::new((8, 12));
        picker.guess_protocol();
        let mut dyn_img = image::io::Reader::open(path_to_image)?.decode()?;
        dyn_img = floyd_steinberg(dyn_img);

        let image = picker.new_resize_protocol(dyn_img);

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
                Tag::GPSLatitude | Tag::GPSLongitude => {
                    has_gps = true;
                    exif_data_rows.push(f.clone());
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
                        let lat_internals = vec![
                            (v[0].num as f32 / v[0].denom as f32),
                            (v[1].num as f32 / v[1].denom as f32) / 60.,
                            (v[2].num as f32 / v[2].denom as f32) / (60. * 100.),
                        ];
                        lat_internals
                            .iter()
                            .fold(0., |sum: f32, x| if x.is_nan() { sum } else { sum + x })
                    }
                    _ => 0.,
                },
                None => 0.,
            };
            let long: f32 = match exif.get_field(Tag::GPSLongitude, In::PRIMARY) {
                Some(l) => match l.value {
                    Value::Rational(ref v) if !v.is_empty() => {
                        let long_internals = vec![
                            (v[0].num as f32 / v[0].denom as f32),
                            (v[1].num as f32 / v[1].denom as f32) / 60.,
                            (v[2].num as f32 / v[2].denom as f32) / (60. * 100.),
                        ];
                        long_internals
                            .iter()
                            .fold(0., |sum: f32, x| if x.is_nan() { sum } else { sum + x })
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

            if lat == 0. && long == 0. {
                has_gps = false
            }
            GPSInfo {
                latitude: lat,
                lat_direction: lat_dir,
                longitude: long,
                long_direction: long_dir,
            }
        } else {
            has_gps = false;
            GPSInfo::default()
        };

        Ok(Self {
            path_to_image: path_to_image.to_path_buf(),
            image,
            exif,
            original_fields: exif_data_rows.clone(),
            modified_fields: exif_data_rows.clone(),
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
            show_keybinds: false,
        })
    }

    pub fn keybind_rows(&self) -> Vec<Row> {
        Vec::from([
            Row::new(vec!["q", "Quit"]),
            Row::new(vec!["r", "Randomize selected Metadata"]),
            Row::new(vec!["R", "Randomize all Metadata"]),
            Row::new(vec!["c | C", "Clear All Metadata"]),
            Row::new(vec!["s | S", "Save modified metadata"]),
            Row::new(vec!["?", "Show/Dismiss Keybind Info"]),
        ])
    }

    pub fn process_rows(&self) -> Vec<Row> {
        let mut exif_data_rows = Vec::new();

        for f in &self.modified_fields {
            let f_val = f.tag.to_string();
            if f_val.len() > 0 {
                match &f.value {
                    Value::Ascii(x) => {
                        if x.iter().all(|x| x.len() > 0) {
                            exif_data_rows.push(Row::new(vec![
                                f.tag.to_string(),
                                f.display_value()
                                    .with_unit(&self.exif)
                                    .to_string()
                                    .trim_matches('"')
                                    .to_string()
                                    .replace("\\x00", ""),
                            ]));
                        } else {
                            exif_data_rows
                                .push(Row::new(vec![f.tag.to_string(), String::from("")]));
                        }
                    }
                    _ => {
                        exif_data_rows.push(Row::new(vec![
                            f.tag.to_string(),
                            f.display_value()
                                .with_unit(&self.exif)
                                .to_string()
                                .trim_matches('"')
                                .to_string()
                                .replace("\\x00", ""),
                        ]));
                    }
                }
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
        for i in 0..self.modified_fields.len() {
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

        match self.modified_fields.get_mut(index) {
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
    }

    pub fn clear_fields(&mut self) {
        self.modified_fields = self
            .modified_fields
            .iter()
            .map(|f| match &f.value {
                Value::Ascii(x) => {
                    let mut empty_vec: Vec<Vec<u8>> = Vec::with_capacity(x.len());
                    for i in x {
                        empty_vec.push(vec![0; i.len()]);
                    }
                    Field {
                        tag: f.tag,
                        ifd_num: f.ifd_num,
                        value: Value::Ascii(empty_vec),
                    }
                }
                Value::Byte(x) => Field {
                    tag: f.tag,
                    ifd_num: f.ifd_num,
                    value: Value::Byte(vec![0; x.len()]),
                },
                Value::Short(x) => Field {
                    tag: f.tag,
                    ifd_num: f.ifd_num,
                    value: Value::Short(vec![0; x.len()]),
                },
                Value::Long(x) => Field {
                    tag: f.tag,
                    ifd_num: f.ifd_num,
                    value: Value::Long(vec![0; x.len()]),
                },
                Value::Rational(x) => Field {
                    tag: f.tag,
                    ifd_num: f.ifd_num,
                    value: Value::Rational(vec![Rational { num: 0, denom: 0 }; x.len()]),
                },
                Value::SByte(x) => Field {
                    tag: f.tag,
                    ifd_num: f.ifd_num,
                    value: Value::SByte(vec![0; x.len()]),
                },
                Value::SShort(x) => Field {
                    tag: f.tag,
                    ifd_num: f.ifd_num,
                    value: Value::SShort(vec![0; x.len()]),
                },
                Value::SLong(x) => Field {
                    tag: f.tag,
                    ifd_num: f.ifd_num,
                    value: Value::SLong(vec![0; x.len()]),
                },
                Value::SRational(x) => Field {
                    tag: f.tag,
                    ifd_num: f.ifd_num,
                    value: Value::SRational(vec![SRational { num: 0, denom: 0 }; x.len()]),
                },
                Value::Float(x) => Field {
                    tag: f.tag,
                    ifd_num: f.ifd_num,
                    value: Value::Float(vec![0.; x.len()]),
                },
                Value::Double(x) => Field {
                    tag: f.tag,
                    ifd_num: f.ifd_num,
                    value: Value::Double(vec![0.; x.len()]),
                },
                _ => f.clone(),
            })
            .collect();
    }

    fn create_copy_file_name(&self) -> PathBuf {
        let mut copy_file_path = self.path_to_image.clone();
        let copy_file_name = copy_file_path.file_name().expect("Valid File Name");
        copy_file_path.set_file_name(format!("copy-{}", copy_file_name.to_str().unwrap()));
        println!("{}", copy_file_path.display());

        copy_file_path
    }

    pub fn save_state(&mut self) -> Result<()> {
        // Zero out all available tags
        // Internals of Exif read_from_container
        // reader.by_ref().take(4096).read_to_end(&mut buf)?;
        // take -> creates an adapter which will read at most "limit" bytes from it
        let exif_buf = self.exif.buf();
        let size_of_exif_buf = exif_buf.len();
        // eprintln!("Size of og exif buf: {}", size_of_exif_buf);

        // Write exif version to a new exif data buffer
        let mut exif_writer = Writer::new();
        let mut new_exif_buf = io::Cursor::new(Vec::new());

        // Modified fields will always have the latest modifications to the state of the
        // Exif Metadata (including randomization and clearing)
        for f in &self.modified_fields {
            exif_writer.push_field(&f);
        }

        // https://github.com/kamadak/exif-rs/blob/a8883a6597f2ba9eb8c9b1cb38bfa61a5cc67837/tests/rwrcmp.rs#L90
        let strips = self.get_strips(In::PRIMARY);
        let tn_strips = self.get_strips(In::THUMBNAIL);
        let tiles = self.get_tiles(In::PRIMARY);
        let tn_jpeg = self.get_jpeg(In::THUMBNAIL);

        if let Some(ref strips) = strips {
            exif_writer.set_strips(strips, In::PRIMARY);
        }
        if let Some(ref tn_strips) = tn_strips {
            exif_writer.set_strips(tn_strips, In::THUMBNAIL);
        }
        if let Some(ref tiles) = tiles {
            exif_writer.set_tiles(tiles, In::PRIMARY);
        }
        if let Some(ref tn_jpeg) = tn_jpeg {
            exif_writer.set_jpeg(tn_jpeg, In::THUMBNAIL);
        }
        exif_writer.write(&mut new_exif_buf, self.exif.little_endian())?;
        let new_exif_buf = new_exif_buf.clone().into_inner();
        eprintln!("Size of new exif buf: {}", new_exif_buf.len());

        // Open the Image File and read into a buffer
        let file = std::fs::File::open(&self.path_to_image)?;
        let mut bufreader = std::io::BufReader::new(&file);
        let mut img_buf = Vec::new();
        _ = bufreader.read_to_end(&mut img_buf);

        // Replace the exif buffer slice in the original image with the one we create
        let position_of_exif = img_buf
            .windows(2)
            .position(|x| x == &new_exif_buf[0..2])
            .unwrap();

        let mut exif_header = Vec::new();
        exif_header.extend_from_slice(&img_buf[0..position_of_exif]);
        exif_header.extend(new_exif_buf.clone());
        // exif_header.extend(exif_buf);
        let img_data = &img_buf[position_of_exif + size_of_exif_buf..];
        exif_header.extend_from_slice(&img_data);
        eprintln!("Position of start of exif: {}", position_of_exif);
        eprintln!("{}", exif_header.len());

        // Create a file copy using the original name of the file
        let mut copy_file = std::fs::File::create(self.create_copy_file_name())?;
        copy_file.write_all(&exif_header.as_slice())?;

        Ok(())
    }

    fn get_strips(&self, ifd_num: In) -> Option<Vec<&[u8]>> {
        let offsets = self
            .exif
            .get_field(Tag::StripOffsets, ifd_num)
            .and_then(|f| f.value.iter_uint());
        let counts = self
            .exif
            .get_field(Tag::StripByteCounts, ifd_num)
            .and_then(|f| f.value.iter_uint());
        let (offsets, counts) = match (offsets, counts) {
            (Some(offsets), Some(counts)) => (offsets, counts),
            (None, None) => return None,
            _ => panic!("inconsistent strip offsets and byte counts"),
        };
        let buf = self.exif.buf();
        assert_eq!(offsets.len(), counts.len());
        let strips = offsets
            .zip(counts)
            .map(|(ofs, cnt)| &buf[ofs as usize..(ofs + cnt) as usize])
            .collect();
        Some(strips)
    }

    fn get_tiles(&self, ifd_num: In) -> Option<Vec<&[u8]>> {
        let offsets = self
            .exif
            .get_field(Tag::TileOffsets, ifd_num)
            .and_then(|f| f.value.iter_uint());
        let counts = self
            .exif
            .get_field(Tag::TileByteCounts, ifd_num)
            .and_then(|f| f.value.iter_uint());
        let (offsets, counts) = match (offsets, counts) {
            (Some(offsets), Some(counts)) => (offsets, counts),
            (None, None) => return None,
            _ => panic!("inconsistent tile offsets and byte counts"),
        };
        assert_eq!(offsets.len(), counts.len());
        let buf = self.exif.buf();
        let strips = offsets
            .zip(counts)
            .map(|(ofs, cnt)| &buf[ofs as usize..(ofs + cnt) as usize])
            .collect();
        Some(strips)
    }

    pub fn get_jpeg(&self, ifd_num: In) -> Option<&[u8]> {
        let offset = self
            .exif
            .get_field(Tag::JPEGInterchangeFormat, ifd_num)
            .and_then(|f| f.value.get_uint(0));
        let len = self
            .exif
            .get_field(Tag::JPEGInterchangeFormatLength, ifd_num)
            .and_then(|f| f.value.get_uint(0));
        let (offset, len) = match (offset, len) {
            (Some(offset), Some(len)) => (offset as usize, len as usize),
            (None, None) => return None,
            _ => panic!("inconsistent JPEG offset and length"),
        };
        let buf = self.exif.buf();
        Some(&buf[offset..offset + len])
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
