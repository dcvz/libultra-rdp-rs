use super::image::ImageSize;
use super::utils::ursi32;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn tex_pad_width(size: ImageSize, line: u32, width: u32) -> i32 {
    if line == 0 {
        return 0;
    }

    let line = line as i32;
    let width = width as i32;
    
    let pad_texels = (line << (4 - size as i32)) - width;
    if let ImageSize::G_IM_SIZ_4b = size {
        ursi32(pad_texels, 1) as i32
    } else {
        pad_texels << (size as i32 - 1)
    }
}
    
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tex_pad_width() {
        let a = tex_pad_width(ImageSize::G_IM_SIZ_8b, 8, 32);
        assert_eq!(a, 32);

        let b = tex_pad_width(ImageSize::G_IM_SIZ_8b, 4, 64);
        assert_eq!(b, -32);
    }
}
