fn main() -> std::io::Result<()> {
    let image_file = std::path::Path::new("./test.jpg");
    if image_file.is_file() {
        println!("Image: {}", image_file.display());
    } else {
        println!("Image not present");
    }

    Ok(())
}
