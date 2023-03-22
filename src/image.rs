use super::utils::I32MathExt;
use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use std::io::Cursor;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::describe::WasmDescribe;
use wasm_bindgen::prelude::wasm_bindgen;

pub enum ImageFormat {
    G_IM_FMT_RGBA = 0x00,
    G_IM_FMT_YUV = 0x01,
    G_IM_FMT_CI = 0x02,
    G_IM_FMT_IA = 0x03,
    G_IM_FMT_I = 0x04,
}

impl ImageFormat {
    pub fn get_name(&self) -> &'static str {
        match self {
            ImageFormat::G_IM_FMT_RGBA => "RGBA",
            ImageFormat::G_IM_FMT_YUV => "YUV",
            ImageFormat::G_IM_FMT_CI => "CI",
            ImageFormat::G_IM_FMT_IA => "IA",
            ImageFormat::G_IM_FMT_I => "I",
        }
    }
}

impl FromWasmAbi for ImageFormat {
    type Abi = u32;

    unsafe fn from_abi(abi: Self::Abi) -> Self {
        match abi {
            0 => ImageFormat::G_IM_FMT_RGBA,
            1 => ImageFormat::G_IM_FMT_YUV,
            2 => ImageFormat::G_IM_FMT_CI,
            3 => ImageFormat::G_IM_FMT_IA,
            4 => ImageFormat::G_IM_FMT_I,
            _ => panic!("Invalid ImageFormat"),
        }
    }
}

impl WasmDescribe for ImageFormat {
    fn describe() {
        u32::describe();
    }
}

#[derive(Clone, Copy)]
pub enum ImageSize {
    G_IM_SIZ_4b = 0x00,
    G_IM_SIZ_8b = 0x01,
    G_IM_SIZ_16b = 0x02,
    G_IM_SIZ_32b = 0x03,
}

impl ImageSize {
    pub fn get_bits_per_pixel(&self) -> u32 {
        match self {
            ImageSize::G_IM_SIZ_4b => 4,
            ImageSize::G_IM_SIZ_8b => 8,
            ImageSize::G_IM_SIZ_16b => 16,
            ImageSize::G_IM_SIZ_32b => 32,
        }
    }

    pub fn get_tlut_size(&self) -> u32 {
        match self {
            ImageSize::G_IM_SIZ_4b => 0x10,
            ImageSize::G_IM_SIZ_8b => 0x100,
            ImageSize::G_IM_SIZ_16b => 0x1000,
            ImageSize::G_IM_SIZ_32b => 0x10000,
        }
    }

    pub fn get_name(&self) -> &'static str {
        match self {
            ImageSize::G_IM_SIZ_4b => "4",
            ImageSize::G_IM_SIZ_8b => "8",
            ImageSize::G_IM_SIZ_16b => "16",
            ImageSize::G_IM_SIZ_32b => "32",
        }
    }
}

impl FromWasmAbi for ImageSize {
    type Abi = u32;

    unsafe fn from_abi(abi: Self::Abi) -> Self {
        match abi {
            0 => ImageSize::G_IM_SIZ_4b,
            1 => ImageSize::G_IM_SIZ_8b,
            2 => ImageSize::G_IM_SIZ_16b,
            3 => ImageSize::G_IM_SIZ_32b,
            _ => panic!("Invalid ImageSize"),
        }
    }
}

impl WasmDescribe for ImageSize {
    fn describe() {
        u32::describe();
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TextureLUT {
    G_TT_NONE = 0x00,
    G_TT_RGBA16 = 0x02,
    G_TT_IA16 = 0x03,
}

impl FromWasmAbi for TextureLUT {
    type Abi = u32;

    unsafe fn from_abi(abi: Self::Abi) -> Self {
        match abi {
            0 => TextureLUT::G_TT_NONE,
            2 => TextureLUT::G_TT_RGBA16,
            3 => TextureLUT::G_TT_IA16,
            _ => panic!("Invalid TextureLUT"),
        }
    }
}

impl WasmDescribe for TextureLUT {
    fn describe() {
        u32::describe();
    }
}

enum TexCM {
    WRAP = 0x00,
    MIRROR = 0x01,
    CLAMP = 0x02,
    MIRROR_CLAMP = 0x03,
}

enum TextFilt {
    G_TF_POINT = 0x00,
    G_TF_AVERAGE = 0x03,
    G_TF_BILERP = 0x02,
}

fn scale3to8(n: u8) -> u8 {
    n * 0x24
}

fn scale4to8(n: u8) -> u8 {
    n * 0x11
}

fn scale5to8(n: u16) -> u8 {
    ((n * 0xFF) / 0x1F) as u8
}

fn r5g5b5a1(dest: &mut [u8], dest_offset: usize, pixel: u16) {
    let a = pixel & 1;
    let r = (pixel & 0xF800) >> 11;
    let g = (pixel & 0x7C0) >> 6;
    let b = (pixel & 0x3E) >> 1;

    dest[dest_offset + 0] = scale5to8(r);
    dest[dest_offset + 1] = scale5to8(g);
    dest[dest_offset + 2] = scale5to8(b);
    dest[dest_offset + 3] = if a > 0 { 0xFF } else { 0 };
}

fn copy_tlut_color(dest: &mut [u8], dest_offset: usize, color_table: &[u8], index: usize) {
    dest[dest_offset + 0] = color_table[(index * 4) + 0];
    dest[dest_offset + 1] = color_table[(index * 4) + 1];
    dest[dest_offset + 2] = color_table[(index * 4) + 2];
    dest[dest_offset + 3] = color_table[(index * 4) + 3];
}

#[wasm_bindgen]
pub fn decode_tex_rgba16(
    dest: &mut [u8],
    source: &[u8],
    source_offset: u32,
    tile_width: u32,
    tile_height: u32,
    line: u32,
    deinterleave: bool,
) {
    let mut dest_index = 0;
    let mut source_index: i32 = 0;
    let mut cursor = Cursor::new(source);
    let pad_width = padding_for_texture(ImageSize::G_IM_SIZ_16b, line, tile_width);

    for y in 0..tile_height {
        let di = if deinterleave { (y & 1) << 2 } else { 0 };
        for _x in 0..tile_width {
            cursor.set_position((source_offset + (source_index as u32 ^ di)) as u64);

            // TODO: What should we do if we try to read past the end of the buffer?
            if let Ok(pixel) = cursor.read_u16::<BigEndian>() {
                r5g5b5a1(dest, dest_index, pixel);
            }

            source_index += 0x02;
            dest_index += 0x04;
        }

        source_index += pad_width;
    }
}

#[wasm_bindgen]
pub fn decode_tex_rgba32(
    dest: &mut [u8],
    source: &[u8],
    source_index: u32,
    tile_width: u32,
    tile_height: u32,
) {
    dest.copy_from_slice(
        &source[source_index as usize..(source_index + (tile_width * tile_height * 4)) as usize],
    );
}

#[wasm_bindgen]
pub fn decode_tex_ci4(
    dest: &mut [u8],
    source: &[u8],
    source_offset: u32,
    tile_width: u32,
    tile_height: u32,
    tlut_color_table: &[u8],
    line: u32,
    deinterleave: bool,
) {
    let mut dest_index = 0;
    let mut source_index: i32 = 0;
    let mut cursor = Cursor::new(source);
    let pad_width = padding_for_texture(ImageSize::G_IM_SIZ_4b, line, tile_width);

    for y in 0..tile_height {
        let di = if deinterleave { (y & 1) << 2 } else { 0 };
        for _x in (0..tile_width).step_by(2) {
            cursor.set_position((source_offset + (source_index as u32 ^ di)) as u64);

            // TODO: What should we do if we try to read past the end of the buffer?
            if let Ok(bit) = cursor.read_u8() {
                copy_tlut_color(
                    dest,
                    dest_index + 0,
                    tlut_color_table,
                    (bit >> 4 & 0x0F) as usize,
                );
                copy_tlut_color(
                    dest,
                    dest_index + 4,
                    tlut_color_table,
                    (bit & 0x0F) as usize,
                );
            }

            source_index += 0x01;
            dest_index += 0x08;
        }

        source_index += pad_width;
    }
}

#[wasm_bindgen]
pub fn decode_tex_ci8(
    dest: &mut [u8],
    source: &[u8],
    source_offset: u32,
    tile_width: u32,
    tile_height: u32,
    tlut_color_table: &[u8],
    line: u32,
    deinterleave: bool,
) {
    let mut dest_index = 0;
    let mut source_index: i32 = 0;
    let mut cursor = Cursor::new(source);
    let pad_width = padding_for_texture(ImageSize::G_IM_SIZ_8b, line, tile_width);

    for y in 0..tile_height {
        let di = if deinterleave { (y & 1) << 2 } else { 0 };
        for _x in 0..tile_width {
            cursor.set_position((source_offset + (source_index as u32 ^ di)) as u64);

            if let Ok(bit) = cursor.read_u8() {
                copy_tlut_color(dest, dest_index + 0, tlut_color_table, bit as usize);
            }

            source_index += 0x01;
            dest_index += 0x04;
        }

        source_index += pad_width;
    }
}

#[wasm_bindgen]
pub fn decode_tex_ia4(
    dest: &mut [u8],
    source: &[u8],
    source_offset: u32,
    tile_width: u32,
    tile_height: u32,
    line: u32,
    deinterleave: bool,
) {
    let mut dest_index = 0;
    let mut source_index: i32 = 0;
    let mut cursor = Cursor::new(source);
    let pad_width = padding_for_texture(ImageSize::G_IM_SIZ_4b, line, tile_width);

    for y in 0..tile_height {
        let di = if deinterleave { (y & 1) << 2 } else { 0 };
        for _x in (0..tile_width).step_by(2) {
            cursor.set_position((source_offset + (source_index as u32 ^ di)) as u64);

            if let Ok(bit) = cursor.read_u8() {
                let i0 = scale3to8(bit >> 5 & 0x07);
                let a0 = if bit >> 4 & 0x01 > 0 { 0xFF } else { 0x00 };

                dest[dest_index + 0] = i0;
                dest[dest_index + 1] = i0;
                dest[dest_index + 2] = i0;
                dest[dest_index + 3] = a0;

                let i1 = scale3to8(bit >> 1 & 0x07);
                let a1 = if (bit & 0x01) != 0 { 0xFF } else { 0x00 };

                dest[dest_index + 4] = i1;
                dest[dest_index + 5] = i1;
                dest[dest_index + 6] = i1;
                dest[dest_index + 7] = a1;
            }

            source_index += 0x01;
            dest_index += 0x08;
        }

        source_index += pad_width;
    }
}

#[wasm_bindgen]
pub fn decode_tex_ia8(
    dest: &mut [u8],
    source: &[u8],
    source_offset: u32,
    tile_width: u32,
    tile_height: u32,
    line: u32,
    deinterleave: bool,
) {
    let mut dest_index = 0;
    let mut source_index: i32 = 0;
    let mut cursor = Cursor::new(source);
    let pad_width = padding_for_texture(ImageSize::G_IM_SIZ_8b, line, tile_width);

    for y in 0..tile_height {
        let di = if deinterleave { (y & 1) << 2 } else { 0 };
        for _x in 0..tile_width {
            cursor.set_position((source_offset + (source_index as u32 ^ di)) as u64);
            let bit = cursor.read_u8().unwrap();

            let i = scale4to8(bit >> 4 & 0x0F);
            let a = scale4to8(bit & 0x0F);

            dest[dest_index + 0] = i;
            dest[dest_index + 1] = i;
            dest[dest_index + 2] = i;
            dest[dest_index + 3] = a;

            source_index += 0x01;
            dest_index += 0x04;
        }

        source_index += pad_width;
    }
}

#[wasm_bindgen]
pub fn decode_tex_ia16(
    dest: &mut [u8],
    source: &[u8],
    source_offset: u32,
    tile_width: u32,
    tile_height: u32,
    line: u32,
    deinterleave: bool,
) {
    let mut dest_index = 0;
    let mut source_index: i32 = 0;
    let mut cursor = Cursor::new(source);
    let pad_width = padding_for_texture(ImageSize::G_IM_SIZ_16b, line, tile_width);

    for y in 0..tile_height {
        let di = if deinterleave { (y & 1) << 2 } else { 0 };
        for _x in 0..tile_width {
            cursor.set_position((source_offset + (source_index as u32 ^ di)) as u64);

            if let Ok(bit) = cursor.read_u16::<BigEndian>() {
                let i = ((bit >> 8) & 0xFF) as u8;
                let a = (bit & 0xFF) as u8;

                dest[dest_index + 0] = i;
                dest[dest_index + 1] = i;
                dest[dest_index + 2] = i;
                dest[dest_index + 3] = a;
            }

            source_index += 0x02;
            dest_index += 0x04;
        }

        source_index += pad_width;
    }
}

#[wasm_bindgen]
pub fn decode_tex_i4(
    dest: &mut [u8],
    source: &[u8],
    source_offset: u32,
    tile_width: u32,
    tile_height: u32,
    line: u32,
    deinterleave: bool,
) {
    let mut dest_index = 0;
    let mut source_index: i32 = 0;
    let mut cursor = Cursor::new(source);
    let pad_width = padding_for_texture(ImageSize::G_IM_SIZ_4b, line, tile_width);

    for y in 0..tile_height {
        let di = if deinterleave { (y & 1) << 2 } else { 0 };
        for _x in (0..tile_width).step_by(2) {
            cursor.set_position((source_offset + (source_index as u32 ^ di)) as u64);

            if let Ok(bit) = cursor.read_u8() {
                let i0 = scale4to8(bit >> 4 & 0x0F);

                dest[dest_index + 0] = i0;
                dest[dest_index + 1] = i0;
                dest[dest_index + 2] = i0;
                dest[dest_index + 3] = i0;

                let i1 = scale4to8(bit & 0x0F);

                dest[dest_index + 4] = i1;
                dest[dest_index + 5] = i1;
                dest[dest_index + 6] = i1;
                dest[dest_index + 7] = i1;
            }

            source_index += 0x01;
            dest_index += 0x08;
        }

        source_index += pad_width;
    }
}

#[wasm_bindgen]
pub fn decode_tex_i8(
    dest: &mut [u8],
    source: &[u8],
    source_offset: u32,
    tile_width: u32,
    tile_height: u32,
    line: u32,
    deinterleave: bool,
) {
    let mut dest_index = 0;
    let mut source_index: i32 = 0;
    let mut cursor = Cursor::new(source);
    let pad_width = padding_for_texture(ImageSize::G_IM_SIZ_8b, line, tile_width);

    for y in 0..tile_height {
        let di = if deinterleave { (y & 1) << 2 } else { 0 };
        for _x in 0..tile_width {
            cursor.set_position((source_offset + (source_index as u32 ^ di)) as u64);

            if let Ok(i) = cursor.read_u8() {
                dest[dest_index + 0] = i;
                dest[dest_index + 1] = i;
                dest[dest_index + 2] = i;
                dest[dest_index + 3] = i;
            }

            source_index += 0x01;
            dest_index += 0x04;
        }

        source_index += pad_width;
    }
}

#[wasm_bindgen]
pub fn parse_tlut(
    dest: &mut [u8],
    source: &[u8],
    mut index: u32,
    size: ImageSize,
    mode: TextureLUT,
) -> u32 {
    // TODO(jstpierre): non-RGBA16 TLUT modes (comes from TEXTLUT field in SETOTHERMODE_H)
    if let TextureLUT::G_TT_RGBA16 = mode {
    } else {
        panic!("Unsupported TLUT mode {:?}", mode);
    }

    let mut cursor = Cursor::new(source);
    let tlut_size = size.get_tlut_size();
    for i in 0..tlut_size {
        cursor.set_position(index as u64);

        let pixel = cursor.read_u16::<BigEndian>().unwrap();
        r5g5b5a1(dest, (i * 4) as usize, pixel);
        index += 0x02;
    }

    tlut_size * 0x02
}

#[wasm_bindgen]
pub fn padding_for_texture(size: ImageSize, line: u32, width: u32) -> i32 {
    if line == 0 {
        return 0;
    }

    let line = line as i32;
    let width = width as i32;

    let pad_texels = (line << (4 - size as i32)) - width;
    if let ImageSize::G_IM_SIZ_4b = size {
        pad_texels.ushr(1) as i32
    } else {
        pad_texels << (size as i32 - 1)
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[wasm_bindgen]
pub fn get_size_bits_per_pixel(size: ImageSize) -> u32 {
    return size.get_bits_per_pixel();
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[wasm_bindgen]
pub fn get_tlut_size(size: ImageSize) -> u32 {
    return size.get_tlut_size();
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[wasm_bindgen]
pub fn get_image_format_name(format: ImageFormat) -> String {
    return format.get_name().to_string();
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[wasm_bindgen]
pub fn get_image_size_name(size: ImageSize) -> String {
    return size.get_name().to_string();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_padding_for_texture() {
        let a = padding_for_texture(ImageSize::G_IM_SIZ_8b, 8, 32);
        assert_eq!(a, 32);

        let b = padding_for_texture(ImageSize::G_IM_SIZ_8b, 4, 64);
        assert_eq!(b, -32);
    }
}
