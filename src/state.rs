use anyhow::Result;
use core::f32;
use exif::{experimental::Writer, Exif, Field, In, Rational, Reader, SRational, Tag, Value};
use ratatui::{layout::Rect, widgets::Row};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, thread::ThreadProtocol, Resize};
use std::{
    collections::HashMap,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

use crate::{
    globe::*,
    order::{self, OrderedTags},
    randomize::RandomMetadata,
    utils,
};

pub type ExifTags = Vec<Field>;

// Metadata
//
// Structure for defining how the metadata should be represented by Bresson
// It should be easier to store this and implement methods to manipulate it
// inside one struct than have it splintered inside State Management
//
// Ordering of Metadata should be available from this module
// Randomizing should be available from this module
// Editing and Clearing should be available from this mdule
//
// We always need to maintain a copy of the original fields so we can restore
// them after making an arbitrary number of changes
//
// The UI should always display the most recently modified value for any row
// We can store this in a different struct field
//
// We also want to maintain some method of ordering the fields that we want to display, which we can do via OrderedTags
//
// Might be easier to store both the original fields and the modified fields as hashmaps of tags -> values and then exclusively use the provided ordering structure
// and hashmap retrieval to get the stored values

#[derive(Debug, Clone)]
pub struct MetadataVal {
    pub field: Field,
    pub changed: bool,
}

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

pub struct Application {
    pub path_to_image: PathBuf,
    pub exif: Exif,
    pub original_fields: HashMap<Tag, MetadataVal>,
    pub modified_fields: HashMap<Tag, MetadataVal>,
    pub randomizer: RandomMetadata,
    pub ordered_tags: OrderedTags,

    pub async_state: ThreadProtocol,
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
    pub show_image: bool,
}

impl Application {
    pub fn new(
        path_to_image: &Path,
        g: Globe,
        app_mode: AppMode,
        tx_worker: Sender<(Box<dyn StatefulProtocol>, Resize, Rect)>,
    ) -> Result<Self> {
        let file = std::fs::File::open(path_to_image)?;

        let mut bufreader = std::io::BufReader::new(&file);
        let exifreader = Reader::new();
        let exif = exifreader.read_from_container(&mut bufreader)?;
        let mut has_gps = false;
        let dyn_img = image::DynamicImage::from(image::open(path_to_image)?);

        // If the picker doesn't work, we should do something to fail over safely
        let mut picker = Picker::from_termios().unwrap();
        picker.guess_protocol();
        picker.background_color = Some(image::Rgb::<u8>([255, 0, 255]));

        let mut exif_data_map = HashMap::new();
        let ordered_tags = OrderedTags::new();
        for f in exif.fields() {
            if f.tag == Tag::GPSLatitude || f.tag == Tag::GPSLongitude {
                has_gps = true;
            }
            if ordered_tags.tags.contains(&f.tag) {
                exif_data_map.insert(
                    f.tag,
                    MetadataVal {
                        field: f.clone(),
                        changed: false,
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
            original_fields: exif_data_map.clone(),
            modified_fields: exif_data_map.clone(),
            ordered_tags,
            randomizer: RandomMetadata::default(),
            async_state: ThreadProtocol::new(tx_worker, picker.new_resize_protocol(dyn_img)),
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
            show_image: true,
        })
    }

    pub fn keybind_rows(&self) -> Vec<Row> {
        Vec::from([
            Row::new(vec!["r", "Randomize selected Metadata"]),
            Row::new(vec!["R", "Randomize all Metadata"]),
            Row::new(vec!["c | C", "Clear All Metadata"]),
            Row::new(vec!["o | O", "Restore Metadata"]),
            Row::new(vec!["s | S", "Save a Copy"]),
            Row::new(vec!["t | T", "Toggle between Thumbnail and Globe"]),
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
        for (_idx, t) in order::EXIF_FIELDS_ORDERED.iter().enumerate() {
            if let Some(m) = self.modified_fields.get(t) {
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
                        _ => match *t {
                            Tag::GPSLatitude => {
                                vec![
                                    self.tag_desc(f),
                                    format!(
                                        "{} {}",
                                        utils::clean_disp(&f.display_value().to_string()),
                                        &f.display_value()
                                    ),
                                ]
                            }
                            Tag::GPSLongitude => {
                                vec![
                                    self.tag_desc(f),
                                    format!(
                                        "{} {}",
                                        utils::clean_disp(&f.display_value().to_string()),
                                        &f.display_value()
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
                Row::new(data.clone()).height(height)
                // let tag = data.get(0).unwrap().clone();
                // let mut val = data.get(1).unwrap().chars();
                // let sub_string = (0..)
                //     .map(|_| val.by_ref().take(term_width as usize).collect::<String>())
                //     .take_while(|s| !s.is_empty())
                //     .collect::<Vec<_>>()
                //     .join("\n");
                // let height = sub_string.chars().filter(|x| *x == '\n').count();
                // Row::new(vec![tag, sub_string]).height(if height == 0 { 1 } else { height as u16 })
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
        let tag_at_index = order::EXIF_FIELDS_ORDERED.get(index).unwrap();
        if let Some(field_in_map) = self.modified_fields.get_mut(&tag_at_index) {
            field_in_map.changed = true;
            match *tag_at_index {
                Tag::DateTimeOriginal | Tag::DateTime | Tag::DateTimeDigitized => {
                    let new_dt = self.randomizer.randomize_datetime();
                    self.sync_date_fields(new_dt);
                    self.status_msg = String::from("Randomized DateTime");
                }
                Tag::GPSLatitude | Tag::GPSLatitudeRef => self.sync_latitude(),
                Tag::GPSLongitude | Tag::GPSLongitudeRef => self.sync_longitude(),
                _ => {
                    if let Some(v) = self.randomizer.randomize_tag(*tag_at_index) {
                        field_in_map.field.value = v.clone();
                        self.status_msg = format!("Randomized {}", tag_at_index.to_string());
                    } else {
                        self.status_msg = format!("Cannot randomize {}", tag_at_index.to_string());
                    }
                }
            }
        }
    }

    fn sync_latitude(&mut self) {
        let (new_lat, lat_dir) = self.randomizer.random_latlong(Cardinal::North);
        for (&t, m) in self.modified_fields.iter_mut() {
            match t {
                Tag::GPSLatitudeRef => {
                    m.field.value = Value::Ascii(vec![lat_dir.bytes().collect()])
                }
                Tag::GPSLatitude => m.field.value = new_lat.clone(),
                _ => {}
            }
        }
    }

    fn sync_longitude(&mut self) {
        let (new_long, long_dir) = self.randomizer.random_latlong(Cardinal::East);
        for (&t, m) in self.modified_fields.iter_mut() {
            match t {
                Tag::GPSLongitudeRef => {
                    m.field.value = Value::Ascii(vec![long_dir.bytes().collect()])
                }
                Tag::GPSLongitude => m.field.value = new_long.clone(),
                _ => {}
            }
        }
    }

    fn sync_date_fields(&mut self, new_dt: String) {
        for (&t, m) in self.modified_fields.iter_mut() {
            match t {
                Tag::DateTime | Tag::DateTimeOriginal | Tag::DateTimeDigitized => {
                    m.field.value = Value::Ascii(vec![Vec::from(new_dt.clone())]);
                }
                _ => {}
            }
        }
    }

    pub fn clear_fields(&mut self) {
        self.modified_fields
            .iter_mut()
            .map(|(_, m)| {
                m.field.value = match m.field.value.clone() {
                    Value::Ascii(x) => {
                        let mut empty_vec: Vec<Vec<u8>> = Vec::with_capacity(x.len());
                        for i in x {
                            empty_vec.push(vec![0; i.len()]);
                        }
                        Value::Ascii(empty_vec)
                    }
                    Value::Byte(x) => Value::Byte(vec![0; x.len()]),
                    Value::Short(x) => Value::Short(vec![0; x.len()]),
                    Value::Long(x) => Value::Long(vec![0; x.len()]),
                    Value::Rational(x) => {
                        Value::Rational(vec![Rational { num: 0, denom: 0 }; x.len()])
                    }
                    Value::SByte(x) => Value::SByte(vec![0; x.len()]),
                    Value::SShort(x) => Value::SShort(vec![0; x.len()]),
                    Value::SLong(x) => Value::SLong(vec![0; x.len()]),
                    Value::SRational(x) => {
                        Value::SRational(vec![SRational { num: 0, denom: 0 }; x.len()])
                    }
                    Value::Float(x) => Value::Float(vec![0.; x.len()]),
                    Value::Double(x) => Value::Double(vec![0.; x.len()]),
                    _ => m.field.value.clone(),
                }
            })
            .collect()
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
        for (_, m) in &self.modified_fields {
            exif_writer.push_field(&m.field);
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
