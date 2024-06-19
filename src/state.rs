use crate::globe::Globe;
use anyhow::Result;
use core::f32;
use exif::{experimental::Writer, Exif, Field, In, Rational, SRational, Tag, Value};
use ratatui::{layout::Rect, widgets::Row};
use ratatui_image::{protocol::StatefulProtocol, Resize};
use std::{
    collections::HashMap,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

use crate::{randomize::RandomMetadata, utils};

const EXIF_FIELDS_ORDERED: [Tag; 67] = [
    Tag::Make,
    Tag::Model,
    Tag::DateTimeOriginal,
    Tag::ExposureTime,
    Tag::ExposureBiasValue,
    Tag::FNumber,
    Tag::PhotographicSensitivity,
    Tag::FocalLength,
    Tag::WhiteBalance,
    Tag::MeteringMode,
    Tag::GPSLatitude,
    Tag::GPSLatitudeRef,
    Tag::GPSLongitude,
    Tag::GPSLongitudeRef,
    Tag::LensModel,
    Tag::Flash,
    Tag::Orientation,
    Tag::XResolution,
    Tag::YResolution,
    Tag::ResolutionUnit,
    Tag::Software,
    Tag::DateTime,
    Tag::YCbCrPositioning,
    Tag::ExposureProgram,
    Tag::ExifVersion,
    Tag::DateTimeDigitized,
    Tag::OffsetTime,
    Tag::OffsetTimeOriginal,
    Tag::OffsetTimeDigitized,
    Tag::ComponentsConfiguration,
    Tag::ShutterSpeedValue,
    Tag::ApertureValue,
    Tag::BrightnessValue,
    Tag::SubjectArea,
    Tag::MakerNote,
    Tag::SubSecTimeOriginal,
    Tag::SubSecTimeDigitized,
    Tag::FlashpixVersion,
    Tag::ColorSpace,
    Tag::PixelXDimension,
    Tag::PixelYDimension,
    Tag::SensingMethod,
    Tag::SceneType,
    Tag::ExposureMode,
    Tag::DigitalZoomRatio,
    Tag::FocalLengthIn35mmFilm,
    Tag::SceneCaptureType,
    Tag::LensSpecification,
    Tag::LensMake,
    Tag::CompositeImage,
    Tag::GPSAltitudeRef,
    Tag::GPSAltitude,
    Tag::GPSTimeStamp,
    Tag::GPSSpeedRef,
    Tag::GPSSpeed,
    Tag::GPSImgDirectionRef,
    Tag::GPSImgDirection,
    Tag::GPSDestBearingRef,
    Tag::GPSDestBearing,
    Tag::GPSDateStamp,
    Tag::GPSHPositioningError,
    Tag::Compression,
    Tag::XResolution,
    Tag::YResolution,
    Tag::ResolutionUnit,
    Tag::JPEGInterchangeFormat,
    Tag::JPEGInterchangeFormatLength,
];

// const OTHER_EXIF_FIELDS: [Tag; 51] = [];

const METADATA_COUNT: usize = 67;

// Step one is taking a given image file and read out some of the super basic metadata about it

#[derive(Debug, Clone, Copy)]
pub enum AppMode {
    CommandLine,
    InteractiveFile,
}

#[derive(Debug, Clone, Copy)]
pub enum RenderState {
    Thumbnail,
    Globe,
}

#[derive(Debug, Clone, Copy)]
pub enum Cardinal {
    North,
    East,
    West,
    South,
}

#[derive(Debug, Clone)]
pub struct MetadataVal {
    field: Field,
    hidden: bool,
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
    pub exif: Exif,
    pub original_fields: ExifTags,
    pub modified_fields: ExifTags,
    pub randomizer: RandomMetadata,
    pub exif_map: HashMap<Tag, MetadataVal>,

    // pub async_state: ThreadProtocol,
    pub render_state: RenderState,

    pub status_msg: String,

    pub globe: Globe,
    pub app_mode: AppMode,
    pub has_gps: bool,
    pub gps_info: GPSInfo,

    pub camera_settings: CameraSettings,
    pub show_keybinds: bool,
    pub should_rotate: bool,
    pub show_globe: bool,
}

impl Application {
    pub fn new(
        path_to_image: &Path,
        g: Globe,
        app_mode: AppMode,
        _tx_worker: Sender<(Box<dyn StatefulProtocol>, Resize, Rect)>,
    ) -> Result<Self> {
        let file = std::fs::File::open(path_to_image)?;

        let mut bufreader = std::io::BufReader::new(&file);
        let exifreader = exif::Reader::new();
        let exif = exifreader.read_from_container(&mut bufreader)?;
        let mut has_gps = false;
        // let mut picker = Picker::from_termios().unwrap();
        // picker.guess_protocol();
        // picker.background_color = Some(Rgb::<u8>([255, 0, 255]));
        // let dyn_img = image::io::Reader::open(path_to_image)?.decode()?;

        let mut exif_data_rows: ExifTags = Vec::new();
        let mut exif_data_map = HashMap::new();
        for f in exif.fields() {
            if f.tag == Tag::GPSLatitude || f.tag == Tag::GPSLongitude {
                has_gps = true;
            }
            if EXIF_FIELDS_ORDERED.binary_search(&f.tag).is_ok() {
                exif_data_rows.push(f.clone());
                exif_data_map.insert(
                    f.tag,
                    MetadataVal {
                        field: f.clone(),
                        hidden: false,
                    },
                );
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
            exif,
            original_fields: exif_data_rows.clone(),
            modified_fields: exif_data_rows.clone(),
            randomizer: RandomMetadata::default(),
            // async_state: ThreadProtocol::new(tx_worker, picker.new_resize_protocol(dyn_img)),
            render_state: RenderState::Globe,
            status_msg: String::new(),
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
            should_rotate: false || !has_gps,
            show_globe: true,
            exif_map: exif_data_map,
        })
    }

    pub fn keybind_rows(&self) -> Vec<Row> {
        Vec::from([
            Row::new(vec!["r", "Randomize selected Metadata"]),
            Row::new(vec!["R", "Randomize all Metadata"]),
            Row::new(vec!["c | C", "Clear All Metadata"]),
            Row::new(vec!["o | O", "Restore Metadata"]),
            Row::new(vec!["s | S", "Save a Copy"]),
            // Row::new(vec!["t | T", "Toggle between Thumbnail and Globe"]),
            Row::new(vec!["g | G", "Toggle Globe Visibility"]),
            Row::new(vec!["<Spc>", "Toggle Globe Rotation"]),
            Row::new(vec!["?", "Show/Dismiss Keybind Info"]),
            Row::new(vec!["q | <Esc>", "Quit"]),
        ])
    }

    fn tag_desc(&self, f: &Field) -> String {
        f.tag
            .description()
            .unwrap_or(&f.tag.to_string())
            .to_string()
    }

    pub fn process_rows(&self, term_width: u16) -> Vec<Row> {
        let mut exif_data_rows = Vec::new();
        for (_idx, t) in EXIF_FIELDS_ORDERED.iter().enumerate() {
            if let Some(m) = self.exif_map.get(t) {
                let f = &m.field;
                let f_val = f.tag.to_string();
                if f_val.len() > 0 {
                    let data_row = match &f.value {
                        Value::Ascii(x) => {
                            if x.iter().all(|x| x.len() > 0) {
                                vec![
                                    self.tag_desc(f),
                                    utils::clean_disp(
                                        &f.display_value().with_unit(&self.exif).to_string(),
                                    ),
                                ]
                            } else {
                                vec![self.tag_desc(f), String::from("")]
                            }
                        }
                        _ => match f.tag {
                            Tag::GPSLatitude => {
                                let lat_dir = self
                                    .modified_fields
                                    .iter()
                                    .find(|f| f.tag == Tag::GPSLatitudeRef)
                                    .unwrap();
                                vec![
                                    self.tag_desc(f),
                                    format!(
                                        "{} {}",
                                        utils::clean_disp(&f.display_value().to_string()),
                                        lat_dir.display_value()
                                    ),
                                ]
                            }
                            Tag::GPSLongitude => {
                                let long_dir = self
                                    .modified_fields
                                    .iter()
                                    .find(|f| f.tag == Tag::GPSLongitudeRef)
                                    .unwrap();
                                vec![
                                    self.tag_desc(f),
                                    format!(
                                        "{} {}",
                                        utils::clean_disp(&f.display_value().to_string()),
                                        long_dir.display_value()
                                    ),
                                ]
                            }
                            _ => {
                                vec![
                                    self.tag_desc(f),
                                    utils::clean_disp(
                                        &f.display_value().with_unit(&self.exif).to_string(),
                                    ),
                                ]
                            }
                        },
                    };
                    exif_data_rows.push(data_row);
                }
            }
        }

        exif_data_rows
            .iter()
            .map(|data| {
                let mut height = 1;
                let total_length: usize = data.iter().map(|d| d.len()).sum();
                if total_length as u16 >= term_width {
                    height += 1
                };
                // let tag = data.get(0).unwrap();
                // let mut val = data.get(1).unwrap().chars();
                // let sub_string = (0..)
                //     .map(|_| val.by_ref().take(term_width as usize).collect::<String>())
                //     .take_while(|s| !s.is_empty())
                //     .map(|s| Line::from(s))
                //     .collect::<Vec<_>>();

                // let lines = Text::from(sub_string);

                // Row::new(vec![
                //     Cell::from(Text::from(tag.to_owned())),
                //     Cell::from(lines),
                // ])
                // .height(height)

                Row::new(data.clone()).height(height)
            })
            .collect::<Vec<Row>>()
    }

    pub fn rotate_globe(&mut self) {
        let globe_rot_speed = 1. / 1000.;
        let cam_rot_speed = 1. / 1000.;
        self.globe.angle += globe_rot_speed;
        self.camera_settings.alpha += cam_rot_speed + (globe_rot_speed / 2.);
    }

    pub fn toggle_globe(&mut self) {
        self.show_globe = !self.show_globe
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
        let field_at_index = self.modified_fields.get_mut(index).unwrap();
        match field_at_index.tag {
            Tag::DateTimeOriginal | Tag::DateTime | Tag::DateTimeDigitized => {
                let new_dt = self.randomizer.randomize_datetime();
                self.sync_date_fields(new_dt);
                self.status_msg = String::from("Randomized DateTime");
            }
            Tag::GPSLatitude | Tag::GPSLatitudeRef => self.sync_latitude(),
            Tag::GPSLongitude | Tag::GPSLongitudeRef => self.sync_longitude(),
            _ => {
                if let Some(v) = self.randomizer.randomize_tag(field_at_index.tag) {
                    field_at_index.value = v;
                    self.status_msg = format!("Randomized {}", field_at_index.tag.to_string());
                } else {
                    self.status_msg =
                        format!("Cannot randomize {}", field_at_index.tag.to_string());
                }
            }
        }
    }

    fn sync_latitude(&mut self) {
        let (new_lat, lat_dir) = self.randomizer.random_latlong(Cardinal::North);
        for f in self.modified_fields.iter_mut() {
            match f.tag {
                Tag::GPSLatitudeRef => f.value = Value::Ascii(vec![lat_dir.bytes().collect()]),
                Tag::GPSLatitude => f.value = new_lat.clone(),
                _ => {}
            }
        }
    }

    fn sync_longitude(&mut self) {
        let (new_long, long_dir) = self.randomizer.random_latlong(Cardinal::East);
        for f in self.modified_fields.iter_mut() {
            match f.tag {
                Tag::GPSLongitudeRef => f.value = Value::Ascii(vec![long_dir.bytes().collect()]),
                Tag::GPSLongitude => f.value = new_long.clone(),
                _ => {}
            }
        }
    }

    fn sync_date_fields(&mut self, new_dt: String) {
        for f in self.modified_fields.iter_mut() {
            match f.tag {
                Tag::DateTime | Tag::DateTimeOriginal | Tag::DateTimeDigitized => {
                    f.value = Value::Ascii(vec![Vec::from(new_dt.clone())]);
                }
                _ => {}
            }
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
        // println!("{}", copy_file_path.display());

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
        // eprintln!("Size of new exif buf: {}", new_exif_buf.len());

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
        // eprintln!("Position of start of exif: {}", position_of_exif);
        // eprintln!("{}", exif_header.len());

        // Create a file copy using the original name of the file
        let copy_file_name = self.create_copy_file_name();
        let mut copy_file = std::fs::File::create(copy_file_name.clone())?;
        copy_file.write_all(&exif_header.as_slice())?;

        self.show_message(format!("Saved a copy - {:?}", copy_file_name).to_owned());

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

    pub fn show_message(&mut self, msg: String) {
        self.status_msg = msg;
    }

    pub fn toggle_rotate(&mut self) {
        self.should_rotate = !self.should_rotate;
    }

    pub fn toggle_keybinds(&mut self) {
        self.show_keybinds = !self.show_keybinds;
    }

    pub fn toggle_render_state(&mut self) {
        match self.render_state {
            RenderState::Globe => self.render_state = RenderState::Thumbnail,
            RenderState::Thumbnail => self.render_state = RenderState::Globe,
        }
    }
}
