use image::Rgba;

const MULTIPLIER: f32 = 0.125;

pub fn clean_disp(dv: &String) -> String {
    dv.trim_matches('"').replace("\\x00", "")
}

fn mean(list: &[i32]) -> f64 {
    let sum: i32 = Iterator::sum(list.iter());
    f64::from(sum) / (list.len() as f64)
}

fn get_luma(pixel: &image::Rgba<u8>) -> u8 {
    let mut list: [i32; 3] = [0; 3];
    list[0] = pixel[0] as i32;
    list[1] = pixel[1] as i32;
    list[2] = pixel[2] as i32;
    let luma = mean(&list.to_vec()) as u8;
    return luma;
}

fn get_adjusted_pixel(old_pixel: image::Rgba<u8>, adjustment: i8) -> u8 {
    let new_pixel = Rgba([old_pixel[0], old_pixel[1], old_pixel[2], 255]);
    let luma = get_luma(&new_pixel) as i32;
    let adjusted: i32 = luma + adjustment as i32;
    let mut newluma = adjusted;
    if newluma < 0 {
        newluma = 0;
    } else if newluma > 255 {
        newluma = 255;
    }
    return newluma as u8;
}
