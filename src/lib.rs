pub mod globe;

use anyhow::Result;
use exif::Tag;
use ratatui::widgets::Row;
use std::path::Path;
// Step one is taking a given image file and read out some of the super basic metadata about it

pub fn get_all_metadata(path_to_image: &Path) -> Result<Vec<Row>> {
    let file = std::fs::File::open(path_to_image)?;
    let mut bufreader = std::io::BufReader::new(&file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader)?;

    let mut exif_data_rows = Vec::new();
    for f in exif.fields() {
        match f.tag {
            Tag::Make | Tag::Model | Tag::DateTime | Tag::ExposureTime | Tag::FNumber => {
                exif_data_rows.push(Row::new(vec![
                    f.tag.to_string(),
                    f.display_value().with_unit(&exif).to_string(),
                ]));
            }
            _ => {}
        }
    }

    Ok(exif_data_rows)
}
