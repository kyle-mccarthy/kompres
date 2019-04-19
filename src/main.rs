mod filter;

use filter::filtering;
use flate2::read::ZlibEncoder;
use flate2::Compression;
use imagequant;
use lodepng::{self, ColorMode, ColorType::PALETTE, CompressSettings, State, RGBA};
use std::env;
use std::io::Read;
use std::os::raw::c_uchar;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: kompres [filename]");
        return;
    }

    let filename = Path::new(&args[1]);

    if !filename.is_file() {
        panic!("Arg must be an existing file");
    }

    // create the output file path - not current customizable
    let output_filename = format!(
        "{}-compressed.{}",
        filename.file_stem().unwrap().to_str().unwrap(),
        filename.extension().unwrap().to_str().unwrap()
    );
    let output_filename = filename.with_file_name(output_filename);

    let file = match lodepng::decode32_file(filename) {
        Ok(file) => file,
        Err(_) => panic!("Could not open file"),
    };

    // ------

    let width = file.width;
    let height = file.height;

    let (palette, pixels) = quantize(&file.buffer, width as usize, height as usize);

    let mut state = make_state();

    add_palette_to_state(&mut state, palette);

    // encode and output final filesize
    match state.encode(&pixels, width, height) {
        Ok(mut out_buffer) => {
            println!("Output buffer size {}", out_buffer.len());
            filtering(&mut out_buffer, width, height, &state.info_png().color);
        }
        Err(_) => {
            panic!("failed to encode the image");
        }
    }

    //    state
    //        .encode_file(output_filename, &pixels, width, height)
    //        .unwrap();
}

// using imagequant quantize the PNG to reduce the file size
// returns the palette and vector of pixels
fn quantize<PixelType: Copy>(
    buffer: &[PixelType],
    width: usize,
    height: usize,
) -> (Vec<RGBA>, Vec<u8>) {
    // quantize
    let mut liq = imagequant::new();
    liq.set_speed(1);
    liq.set_quality(70, 99);

    let ref mut img = liq
        .new_image(&buffer, width as usize, height as usize, 0.45455)
        .unwrap();

    let mut res = match liq.quantize(img) {
        Ok(res) => res,
        Err(_) => panic!("Failed to quantize image"),
    };

    res.remapped(img).unwrap()
}

// create the initial state with default settings for PNG with a palette
// use flate2 for the image compression rather than the compression that comes with the
// lonepng package, flate2 tends to be significantly faster as well as produces a smaller image
fn make_state() -> State {
    let mut state = lodepng::ffi::State::new();
    state.info_png_mut().color.colortype = PALETTE;
    state.info_png_mut().color.set_bitdepth(8);

    state.info_raw_mut().colortype = PALETTE;
    state.info_raw_mut().set_bitdepth(8);

    // lib uses custom deflate which is slower and creates a larger filesize than flate2
    unsafe {
        state.set_custom_zlib(Some(deflate_ffi), std::ptr::null());
    }

    state.encoder.add_id = 0;
    state.encoder.text_compression = 1;

    state
}

// add the palette from the quantization to the image state
fn add_palette_to_state(state: &mut State, palette: Vec<RGBA>) {
    palette.iter().for_each(|palette| {
        state
            .info_png_mut()
            .color
            .palette_add(palette.clone())
            .unwrap();

        state.info_raw_mut().palette_add(palette.clone()).unwrap();
    });
}

// to override the default compressor for lodepng, an unsafe external c function has to be passed to used
unsafe extern "C" fn deflate_ffi(
    out: &mut *mut c_uchar,
    out_size: &mut usize,
    input: *const c_uchar,
    input_size: usize,
    settings: *const CompressSettings,
) -> u32 {
    let input = vec_from_raw(input, input_size);
    let settings = std::ptr::read(settings);

    let (mut buffer, size) = deflate(&input, settings);

    std::mem::replace(out, buffer.as_mut_ptr());
    std::ptr::replace(out_size, size);

    return 0;
}

// call flate2's zlib encoder return the buffer and length
fn deflate(input: &[u8], _settings: CompressSettings) -> (Vec<u8>, usize) {
    let mut z = ZlibEncoder::new(input, Compression::best());
    let mut buffer = vec![];
    match z.read_to_end(&mut buffer) {
        Ok(len) => (buffer, len),
        Err(_) => panic!("Failed to compress buffer"),
    }
}

// convert the raw buffer to a vector
unsafe fn vec_from_raw(data: *const c_uchar, len: usize) -> Vec<u8> {
    std::slice::from_raw_parts(data, len).to_owned()
}
