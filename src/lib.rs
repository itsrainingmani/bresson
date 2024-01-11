pub mod globe;

use anyhow::Result;
use exif::{Exif, Field, Tag};
use globe::Globe;
use ratatui::widgets::Row;
use std::path::{Path, PathBuf};
// Step one is taking a given image file and read out some of the super basic metadata about it

pub struct ExifMetadata {
    path_to_image: PathBuf,
    exif_data: Vec<Field>,
    exif: Exif,
    pub globe: Globe,
}

impl ExifMetadata {
    pub fn new(path_to_image: &Path, g: Globe) -> Result<Self> {
        let file = std::fs::File::open(path_to_image)?;
        let mut bufreader = std::io::BufReader::new(&file);
        let exifreader = exif::Reader::new();
        let exif = exifreader.read_from_container(&mut bufreader)?;

        let mut exif_data_rows = Vec::new();
        for f in exif.fields() {
            match f.tag {
                Tag::Make
                | Tag::Model
                | Tag::DateTime
                | Tag::ExposureTime
                | Tag::FNumber
                | Tag::FocalLength
                | Tag::GPSLatitude
                | Tag::GPSLongitude => {
                    exif_data_rows.push(f.clone());
                }
                _ => {}
            }
        }

        Ok(Self {
            path_to_image: path_to_image.to_path_buf(),
            exif_data: exif_data_rows,
            exif,
            globe: g,
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
}
