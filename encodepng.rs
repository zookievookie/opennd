use std::path::*;
use png::*;

pub fn save_png(filename: PathBuf, data: Vec<u8>, width: u16, height: u16, output_dir: PathBuf) {
    let output_dir = PathBuf::from(&output_dir).join(format!(
        "{}.png", Path::new(&filename).file_stem().unwrap().to_str().unwrap()
    ));
    encode_png_rgba(&mut rgb555_to_rgba(&data), output_dir, width, height);
}

pub fn save_png_multiple(filename: PathBuf, data: Vec<u8>, width: u16, height: u16, frame_number: u16, output_dir: PathBuf) {
    let output_dir = PathBuf::from(&output_dir).join(format!(
        "{}_{}.png", Path::new(&filename).file_stem().unwrap().to_str().unwrap(), frame_number
    ));
    encode_png_rgba(&mut rgb555_to_rgba(&data), output_dir, width, height);
}

pub fn encode_png_rgba(pixels: &mut Vec<u8>, write:PathBuf, width: u16, height: u16) {
    for i in (0..pixels.len()).step_by(4) {
        let temp: u8 = pixels[i];
        pixels[i] = pixels[i+2];
        pixels[i+2] = temp; 
    }
    std::fs::create_dir_all(write.parent().unwrap()).unwrap();
    while pixels.len() < width as usize *height as usize *4{
        pixels.push(0x0);
    }
    
    let w = std::io::BufWriter::new(std::fs::File::create(std::path::Path::new(&write)).unwrap());
    let mut encoder = Encoder::new(w, width as u32,height as u32);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&pixels).unwrap_or_else(|error| {
        println!("{} on file {:?}", error, write);
    });
}

fn rgb555_to_rgba(rgb555_array: &[u8]) -> Vec<u8>{
    let mut rgba_array = Vec::new();
    let mut pixel_pointer = 0;

    while pixel_pointer < (rgb555_array.len() - 1) {
        let pixel = crate::avf::read_le(&rgb555_array, pixel_pointer, 2); // stored as LE, convert it

        if pixel == 0b0_00000_11111_00000 {
            rgba_array.push(0x00);rgba_array.push(0x00);
            rgba_array.push(0x00);rgba_array.push(0x00);
            pixel_pointer += 2;
            continue;
        }
        else {
            rgba_array.push(((pixel & 0b0_00000_00000_11111) << 3) as u8); // mask the BLUE channel, bit shift...
            rgba_array.push(((pixel & 0b0_00000_11111_00000) >> 2) as u8); // mask the GREEN channel, bit shift...
            rgba_array.push(((pixel & 0b0_11111_00000_00000) >> 7) as u8); // mask the RED channel, bit shift to give 64 intensity levels
            rgba_array.push(0xff);
            pixel_pointer += 2; // increment to next pixels
        }
    }
    return rgba_array.to_vec();
}
