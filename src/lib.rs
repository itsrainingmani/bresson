use anyhow::Result;
use exif::{In, Tag};
use std::path::Path;
// use little_exif::metadata::Metadata;

// Step one is taking a given image file and read out some of the super basic metadata about it

pub fn read_metadata(path_to_image: &Path) -> Result<()> {
    println!("Reading {}", path_to_image.display());
    let file = std::fs::File::open(path_to_image)?;
    let mut bufreader = std::io::BufReader::new(&file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader)?;
    for f in exif.fields() {
        println!("{} {}", f.tag, f.display_value().with_unit(&exif));
    }

    match exif.get_field(Tag::GPSLatitude, In::PRIMARY) {
        Some(gps_coords) => {
            println!(
                "{} {}",
                gps_coords.tag,
                gps_coords.display_value().with_unit(&exif)
            );
        }
        None => {}
    }
    Ok(())
}
