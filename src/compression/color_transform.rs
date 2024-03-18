/// Losslessy convert from the RGB color space to the YCoCg-R color space.
/// This esentially decorrelates the channels and improves compression.
///
/// For RGB signals with depth n, the bit depth of the Y signal will be n and the
/// bit depth of Co and Cg will be n+1.
/// Because we encode images with a bit depth of 8/16, we use i32 as an internal representation
/// so that overflows do not occur during this process.
///
/// Link to paper:
/// [Reversible color transform](https://www.researchgate.net/publication/252965845_Lifting-based_reversible_color_transformations_for_image_compression.)
fn rgb_to_ycocg(r: i32, g: i32, b: i32) -> (i32, i32, i32) {
    let co = r - b;
    let t = b + co / 2;
    let cg = g - t;
    let y = t + cg / 2;
    (y, co, cg)
}

/// The opposite of `rgb_to_ycocg`.
fn ycocg_to_rgb(y: i32, co: i32, cg: i32) -> (i32, i32, i32) {
    let t = y - cg / 2;
    let g = cg + t;
    let b = t - co / 2;
    let r = b + co;
    (r, g, b)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_color_transform8() {
        for r in 0..u8::MAX {
            for g in 0..u8::MAX {
                for b in 0..u8::MAX {
                    let (y, co, cg) = rgb_to_ycocg(r.into(), g.into(), b.into());
                    let (rn, gn, bn) = ycocg_to_rgb(y, co, cg);

                    let rn: u8 = rn.try_into().unwrap();
                    let gn: u8 = gn.try_into().unwrap();
                    let bn: u8 = bn.try_into().unwrap();

                    assert_eq!(rn, r);
                    assert_eq!(gn, g);
                    assert_eq!(bn, b);
                }
            }
        }
    }
    #[test]
    fn test_color_transform16() {
        let values = [
            (0, u16::MAX, 0),
            (0, 0, u16::MAX),
            (u16::MAX, u16::MAX, u16::MAX),
            (u16::MAX, 0, u16::MAX),
            (1726, 12640, 26649),
            (0, 0, 0),
            (9127, 65535, 3),
        ];

        for (r, g, b) in values {
            let (y, co, cg) = rgb_to_ycocg(r.into(), g.into(), b.into());
            let (rn, gn, bn) = ycocg_to_rgb(y, co, cg);

            println!("{} {} {}", y, co, cg);

            let rn: u16 = rn.try_into().unwrap();
            let gn: u16 = gn.try_into().unwrap();
            let bn: u16 = bn.try_into().unwrap();

            assert_eq!(rn, r);
            assert_eq!(gn, g);
            assert_eq!(bn, b);
        }
    }
}
