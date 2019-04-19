use lodepng::ColorMode;

// adopted from https://github.com/PistonDevelopers/image-png/blob/master/src/filter.rs

pub enum FilterType {
    None,
    Sub,
    Up,
    Avg,
    Paeth,
}

pub(crate) fn filtering(buffer: &mut [u8], width: usize, _height: usize, info: &ColorMode) {
    let bpp = info.bpp();
    let row_length: usize = ((width as u32 * bpp + 7) / 8) as usize;

    let curr = &buffer[..row_length];
    let prev: Option<&[u8]> = None;
}

fn filter_sub() {}

fn filter_up() {}

fn filter_avg() {}

fn filter_paeth() {}
