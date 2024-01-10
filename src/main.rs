use anyhow::Result;
use bresson::*;
use std::path::Path;

fn main() -> Result<()> {
    let image_arg = std::env::args().nth(1).unwrap();
    let image_file = Path::new(&image_arg);
    if image_file.is_file() {
        println!("Image: {}", image_file.display());
    } else {
        println!("Image not present");
    }

    read_metadata(image_file)?;

    Ok(())
}
