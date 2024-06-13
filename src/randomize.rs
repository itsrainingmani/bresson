use std::collections::HashSet;

use chrono::{Datelike, Timelike, Utc};
use exif::{Tag, Value};
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};

use crate::state::Cardinal;

const MANUFACTURERS: [&str; 48] = [
    "Acer",
    "Apple",
    "BenQ",
    "BlackBerry",
    "Canon",
    "Casio",
    "Concord",
    "DJI",
    "DoCoMo",
    "Epson",
    "Fujifilm",
    "GoPro",
    "Google",
    "HP",
    "HTC",
    "Hasselblad",
    "Helio",
    "Huawei",
    "JVC",
    "KDDI",
    "Kodak",
    "Konica Minolta",
    "Kyocera",
    "LG",
    "Leaf",
    "Leica",
    "Mamiya",
    "Motorola",
    "Nikon",
    "Nintendo",
    "Nokia",
    "Olympus",
    "OnePlus",
    "Palm",
    "Panasonic",
    "Pentax",
    "Phase One",
    "Polaroid",
    "Ricoh",
    "Samsung",
    "Sanyo",
    "Sharp",
    "Sigma",
    "Sony",
    "Sony Ericsson",
    "Toshiba",
    "Vivitar",
    "Xiaomi",
];

const F_NUMBERS: [f32; 13] = [
    1.0, 1.4, 2.0, 2.8, 4.0, 5.6, 8.0, 11.0, 16.0, 22.0, 32.0, 45.0, 64.0,
];

pub struct RandomMetadata {
    pub tags_to_randomize: HashSet<Tag>,
    thread_rng: ThreadRng,
}

impl Default for RandomMetadata {
    fn default() -> Self {
        Self {
            tags_to_randomize: HashSet::from([
                Tag::Make,
                Tag::Model,
                Tag::DateTimeOriginal,
                Tag::ExposureTime,
                Tag::FNumber,
                Tag::MeteringMode,
                Tag::ColorSpace,
                Tag::GPSLatitude,
                Tag::GPSLatitudeRef,
                Tag::GPSLongitude,
                Tag::GPSLongitudeRef,
                Tag::DateTime,
                Tag::DateTimeDigitized,
            ]),
            thread_rng: rand::thread_rng(),
        }
    }
}

impl RandomMetadata {
    pub fn randomize_datetime(&mut self) -> String {
        let now_utc = Utc::now();
        let date_utc = now_utc.date_naive();
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.thread_rng.gen_range(2001..=date_utc.year_ce().1),
            self.thread_rng.gen_range(1..=(date_utc.month0() + 1)),
            self.thread_rng.gen_range(1..=(date_utc.day0() + 1)),
            self.thread_rng.gen_range(0..=now_utc.hour()),
            self.thread_rng.gen_range(0..=now_utc.minute()),
            self.thread_rng.gen_range(0..=now_utc.second())
        )
    }

    pub fn randomize_tag(&mut self, tag_to_modify: Tag) -> Option<Value> {
        // let mut random_data: ExifTags = Vec::new();
        if self.tags_to_randomize.contains(&tag_to_modify) {
            match tag_to_modify {
                Tag::Make => Some(Value::Ascii(vec![Vec::from(
                    *MANUFACTURERS.choose(&mut self.thread_rng).unwrap(),
                )])),
                Tag::ExposureTime => Some(Value::Rational(vec![exif::Rational {
                    num: 1,
                    denom: rand::random::<u8>() as u32,
                }])),
                Tag::FNumber => Some(Value::Float(vec![*F_NUMBERS
                    .choose(&mut self.thread_rng)
                    .unwrap()])),
                Tag::MeteringMode => Some(Value::Short(vec![self.thread_rng.gen_range(1..=6)])),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn random_latlong(&mut self, direction: Cardinal) -> (Value, String) {
        let latlong_range = match direction {
            Cardinal::East | Cardinal::West => 180,
            Cardinal::North | Cardinal::South => 90,
        };
        let new_lat_deg = self.thread_rng.gen_range(0..latlong_range);
        let new_lat_min = self.thread_rng.gen_range(0..60);
        let new_lat_sec = self.thread_rng.gen_range(0..60);

        let dir_rand = self.thread_rng.gen_bool(0.5);
        let dir = match direction {
            Cardinal::East | Cardinal::West => {
                if dir_rand {
                    String::from('E')
                } else {
                    String::from('W')
                }
            }
            Cardinal::North | Cardinal::South => {
                if dir_rand {
                    String::from('N')
                } else {
                    String::from('S')
                }
            }
        };

        let new_lat = Value::Rational(vec![
            (new_lat_deg, 1).into(),
            (new_lat_min, 1).into(),
            (new_lat_sec, 1).into(),
        ]);

        (new_lat, dir)
    }
}
