pub fn ursi32(x: i32, n: u32) -> u32 {
    let x_as_u32 = {
        let bytes = x.to_be_bytes();
        i32::from_be_bytes(bytes)
    };

    (x_as_u32 >> n) as u32
}

pub fn ursi16(x: i16, n: u16) -> u16 {
    let x_as_u32 = {
        let bytes = x.to_be_bytes();
        i16::from_be_bytes(bytes)
    };

    (x_as_u32 >> n) as u16
}

pub fn ursi8(x: i8, n: u8) -> u8 {
    let x_as_u8 = {
        let bytes = x.to_be_bytes();
        i8::from_be_bytes(bytes)
    };

    (x_as_u8 >> n) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unsigned_right_shift() {
        const a: i32 = 5;
        const b: u32 = 2;
        const c: i32= -5;

        assert_eq!(ursi32(a, b), 1);
        assert_eq!(ursi32(c, b), 1073741822);
        assert_eq!(ursi32(9, 2), 2);
        assert_eq!(ursi32(-9, 2), 1073741821);
    }
}
