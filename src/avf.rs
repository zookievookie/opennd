use std::io::*;
use std::path::*;
use std::num::Wrapping;
use std::thread;

use crate::encodepng::*;

pub struct ChunkHeader {
    pub file_id: String,
    pub version: u16,
    pub revision: u16,
    pub chunk_type: u8,
    pub num_chunks: i16,
    pub width: i16,
    pub height: i16,
    pub bits_per_pixel: u8,
    pub time_per_frame: u32,
    pub compression_mode: u8
}
pub struct ChunkInfoBlk {
    pub info_block_offset: u16,
    pub file_offset: usize,
    pub storage_size: usize,
    pub original_size: usize,
    pub chunk_type: u8,
    pub parent_key_frame: u32
}

pub fn avf_to_png(input_file: PathBuf, mut output_dir: PathBuf){
    let mut data: Vec<u8> = Vec::new();
    std::fs::File::open(&input_file).unwrap().read_to_end(&mut data).unwrap();
    if data.len() == 0 {return;} // sanity check

    let header: ChunkHeader = get_header(&data);
    if header.file_id != "AVF WayneSikes\0".to_string() {
        println!("Invalid file header: {}", header.file_id);
        panic!();
    }
    if header.num_chunks > 1 {
        output_dir.push(input_file.file_stem().unwrap().to_str().unwrap());
        std::fs::create_dir_all(&output_dir).unwrap();
    }

    let frames_info: Vec<ChunkInfoBlk> = build_chunk_database(&data, header.num_chunks); // generates cache of chunk info

    let frame_size: usize = (header.height as usize * header.width as usize * 2) as usize;
    let mut images: Vec<Vec<u8>> = vec![vec![0; frame_size]; header.num_chunks as usize];
    
    for f in 0..header.num_chunks as usize {
        let comp_chunk_data: &mut [u8] = &mut data[frames_info[f].file_offset..(frames_info[f].file_offset + frames_info[f].storage_size)];

        for n in 0..comp_chunk_data.len() {                  // unencrypt the data
            comp_chunk_data[n] = (Wrapping(comp_chunk_data[n]) - Wrapping((n % 256) as u8)).0;
        }
        let chunk_data: Vec<u8> = decode_lzss(comp_chunk_data); // decompress the data
        if chunk_data.len() == 0 {break};
        
        if (frames_info[f].original_size == 0) && (frames_info[f].storage_size == 0) {
            images[f] = images[f-1].clone();        // if this chunk is empty, it's a duplicate
            continue;
        }

        if frames_info[f].chunk_type == 2 {       // special treatment for FDD frames
            images[f] = decode_frame(
                &chunk_data,
                frames_info[f].chunk_type,
                frame_size,
                &images[f-1].clone()     // supply the reference image
            );
        }
        else {
            images[f] = decode_frame(   // unoptimized and keyframes don't need references
                &chunk_data,
                frames_info[f].chunk_type,
                frame_size,
                &vec![0; frame_size] // supply a blank vector
            );
        }

        if header.num_chunks > 1 {  // if this file has more than one chunk, create a folder
            // use multithreading here

            let filename = input_file.clone(); 
            let rgbarray = images[f].clone();
            let outpath = output_dir.clone();
            thread::spawn(move || {
                save_png_multiple(
                    filename, 
                    rgbarray,
                    header.width as u16, header.height as u16, f as u16, 
                    outpath
                );
            });

        }
        else {
            save_png(    // save normally w/o multithreading
                input_file.clone(),
                images[f].clone(),
                header.width as u16, header.height as u16,
                output_dir.clone()
            )
        }
    }
}

fn build_chunk_database(data: &Vec<u8>, num_chunks: i16) -> Vec<ChunkInfoBlk> {
    let mut index: usize = 0x21; // size of ChunkHeader struct
    let mut chunk_database: Vec<ChunkInfoBlk> = Vec::new();

    for _i in 0..num_chunks as usize{
        chunk_database.push(ChunkInfoBlk {
            info_block_offset: read_le(&data,index + 0x00, 2) as u16,
            file_offset: read_le(&data,index + 0x02, 4),
            storage_size: read_le(&data, index + 0x06, 4),
            original_size: read_le(&data, index + 0x0a, 4),
            chunk_type: data[index + 0x0e],
            parent_key_frame: read_le(&data, index + 0x0f, 4) as u32
        });
        index += 0x13; // size of ChunkInfoBlk struct
    }
    return chunk_database;
}

fn decode_frame(data: &Vec<u8>, chunk_type: u8, frame_size: usize, ref_frame: &Vec<u8>) -> Vec<u8> {
    if chunk_type == 0 {
        return data.to_vec();
    };

    let mut i: usize = 0;

    let mut decoded_frame: Vec<u8> = vec![0; frame_size]; // initialize a vector for all pixels

    if chunk_type == 2 {decoded_frame = ref_frame.to_vec()} // pre-fill the output vector with a reference picture

    while i < data.len() {
        match data[i] {
            0x20 => { // this operation will copy pixel data starting at [i+9] to the offset location in the final image
                let offset: usize = 2 * read_le(data, i + 0x01, 4);
                let num_pixels: usize = 2 * read_le(data, i + 0x05, 4);
                i += 9;
                for p in 0..num_pixels{
                    decoded_frame[offset + p] = data[i + p];
                }
                i += num_pixels;
            },
            0x40 => { // this operation will repeat the same pixel n times at the offset location in the final image
                let upper_byte: u8 = data[i + 1];
                let lower_byte: u8 = data[i + 2];
                let offset: usize = 2 * read_le(&data, i + 3, 4);
                let num_repetitions: usize = 2 * read_le(&data, i + 7, 4);

                i += 11;
                
                for _x in (0..num_repetitions).step_by(2) {
                    decoded_frame[(offset + _x)] = upper_byte;
                    decoded_frame[(offset + _x + 1)] = lower_byte;
                };
            },
            0x80 => {
                let num_pixels_in_group: usize = 2 * data[(i + 0x01)] as usize;
                let num_offset_repeats: usize = read_le(data, i + 0x02, 4);
                let mut offset: usize = 0;
                
                for _i in 0..num_offset_repeats {
                    for n in 0..num_pixels_in_group {
                        decoded_frame[(2 * offset) + n] = 0;
                    }
                    decoded_frame[2 * offset] = data[i + num_pixels_in_group as usize];
                    offset += 4;
                    i += 0x07;
                }
                eprintln!{"RepPixelGrp @{:#10x} size {:#10x} {:#10x}x", i, num_pixels_in_group, num_offset_repeats}; // WIP it may work it may not.
            },
            _ => {
                eprintln!("Optimization flag {} not recognized at {}",data[i], i); // WIP: the decoder occasionally comes across 0x00??
                break;
            }
        }
    };
    return decoded_frame;
}

fn get_header(data: &Vec<u8>) -> ChunkHeader {
    let header: ChunkHeader = ChunkHeader {
        file_id: String::from_utf8_lossy(&data[0x00..0x0f].to_vec()).to_string(),
        version: read_le(&data, 0x10, 2) as u16,
        revision: read_le(&data, 0x12, 2) as u16,
        chunk_type: data[0x14],
        num_chunks: read_le(&data, 0x15, 2) as i16,
        width: read_le(&data, 0x17, 2) as i16,
        height: read_le(&data, 0x19, 2) as i16,
        bits_per_pixel: data[0x1b],
        time_per_frame: read_le(&data, 0x1c, 4) as u32,
        compression_mode: data[0x20]
    };
    return header;
}

pub fn read_le(data: &[u8], start: usize, length: usize) -> usize {
    let mut output_bytes: usize = 0;
    for bytes in 0..length {
        output_bytes <<= 8;
        output_bytes |= data[start + length - bytes - 1] as usize;
    }
    return output_bytes;
}
fn decode_lzss(data: &mut [u8]) -> Vec<u8> {
    let mut output: Vec<u8> = Vec::new();
    let mut buffer: [u8; 4096] = [0x0; 4096];
    let mut flags: u8;
    let mut buf_write_index: u16 = 0xFEE;
    let mut buf_read_index: u16;
    let mut index = 0;

    while index < data.len() {
        flags = data[index];
        index += 1;
        for _i in 0..8 {
            if (flags & 1) != 0 {
                output.push(data[index]);
                buffer[buf_write_index as usize] = data[index];
                buf_write_index += 1;
                buf_write_index %= 4096;
                index += 1;
            } else {
                buf_read_index = data[index] as u16;
                index += 1;
                buf_read_index |= ((data[index] & 0xF0) as u16) << 4;
                let mut j = 0;
                while j < (data[index] & 0x0f) + 3 {
                    output.push(buffer[buf_read_index as usize]);
                    buffer[buf_write_index as usize] = buffer[buf_read_index as usize];
                    buf_read_index += 1;
                    buf_read_index %= 4096;
                    buf_write_index += 1;
                    buf_write_index %= 4096;
                    j += 1;
                }
                index += 1;
            }
            flags >>= 1;
            if index >= data.len() {
                break;
            }
        }
    }
    return output;
}