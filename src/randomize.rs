use std::collections::HashSet;

use chrono::{Datelike, Timelike, Utc};
use exif::{Tag, Value};
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};

const MANUFACTURERS: [&str; 20] = [
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

const F_NUMBERS: [u32; 12] = [1, 2, 3, 4, 5, 8, 11, 16, 22, 32, 45, 64];

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
                Tag::DateTime,
                Tag::DateTimeDigitized,
                Tag::ExposureTime,
                Tag::FNumber,
                Tag::MeteringMode,
                Tag::ColorSpace,
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
                Tag::FNumber => Some(Value::Rational(vec![exif::Rational {
                    num: *F_NUMBERS.choose(&mut self.thread_rng).unwrap(),
                    denom: self.thread_rng.gen_range(1..=3),
                }])),
                Tag::MeteringMode => Some(Value::Short(vec![self.thread_rng.gen_range(1..=6)])),
                _ => None,
            }
        } else {
            None
        }
    }
}
