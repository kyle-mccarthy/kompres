use flate2::read::ZlibEncoder;
use flate2::Compression;
use imagequant;
use lodepng;
use lodepng::ffi::ColorType::PALETTE;
use lodepng::CompressSettings;
use std::env;
use std::io::Read;
use std::os::raw::c_uchar;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

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
        Err(e) => panic!("Could not open file"),
    };

    let width = file.width;
    let height = file.height;

    let buffer = file.buffer.clone();

    // quantize
    let mut liq = imagequant::new();
    liq.set_speed(1);
    liq.set_quality(70, 99);

    let ref mut img = liq
        .new_image(&buffer, width as usize, height as usize, 0.45455)
        .unwrap();

    let mut res = match liq.quantize(img) {
        Ok(res) => res,
        Err(e) => panic!("Failed to quantize image"),
    };

    //    res.set_dithering_level(1.0);

    let (palette, pixels) = res.remapped(img).unwrap();

    // set up the encoder
    // https://github.com/ImageOptim/libimagequant/blob/f54d2f1a3e1cf728e17326f4db0d45811c63f063/example.c
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

    //    state.encoder.filter_strategy = FilterStrategy::BRUTE_FORCE;

    // add the pallets from the quantization
    palette.iter().for_each(|palette| {
        state
            .info_png_mut()
            .color
            .palette_add(palette.clone())
            .unwrap();

        state.info_raw_mut().palette_add(palette.clone()).unwrap();
    });

    // encode and output final filesize
    match state.encode(&pixels, width, height) {
        Ok(out_buffer) => {
            println!("Output buffer size {}", out_buffer.len());
        }
        Err(e) => {
            println!("failed to encode the image");
        }
    }

    state
        .encode_file(output_filename, &pixels, width, height)
        .unwrap();
}

unsafe extern "C" fn deflate_ffi(
    out: &mut *mut c_uchar,
    out_size: &mut usize,
    input: *const c_uchar,
    input_size: usize,
    arg5: *const CompressSettings,
) -> u32 {
    let input = vec_from_raw(input, input_size);
    let settings = std::ptr::read(arg5);

    let (mut buffer, size) = deflate(&input, settings);

    std::mem::replace(out, buffer.as_mut_ptr());
    std::ptr::replace(out_size, size);

    return 0;
}

fn deflate(input: &[u8], settings: CompressSettings) -> (Vec<u8>, usize) {
    let mut z = ZlibEncoder::new(input, Compression::best());
    let mut buffer = vec![];
    match z.read_to_end(&mut buffer) {
        Ok(len) => (buffer, len),
        Err(e) => panic!("Failed to compress buffer"),
    }
}

unsafe fn vec_from_raw(data: *const c_uchar, len: usize) -> Vec<u8> {
    std::slice::from_raw_parts(data, len).to_owned()
}
