use crate::coding::{phase_in_coding::PhaseInCoder, rice_coding::RiceCoder};
use bitstream_io::{self, BigEndian, BitRead, BitReader, BitWrite, BitWriter};
use color_transform::{rgb_to_ycocg, ycocg_to_rgb};
pub use error::DecompressionError;
pub use format::{read_header, write_header, ColorType, Header, PixelDepth};
use image::{DynamicImage, ImageBuffer, Luma, Pixel, Rgb};
use parameter_selection::KEstimator;
use std::cmp;
use std::io::{self, Read, Write};
pub use traits::{CompressDecompress, Intensity};

mod color_transform;
mod error;
mod format;
mod misc;
mod parameter_selection;
mod traits;

/// The possible intensity of a pixel relative to the context induced by its two
/// nearest neighbours: `[L, H]`.
#[derive(PartialEq, Debug)]
enum PixelIntensity {
    InRange,
    BelowRange,
    AboveRange,
}

/// Writes the `PixelIntensity` to the given `BitWrite` using simple prefix codes.
fn encode_intensity<T>(bitwrite: &mut T, intensity: PixelIntensity) -> io::Result<()>
where
    T: BitWrite,
{
    match intensity {
        PixelIntensity::InRange => bitwrite.write_bit(true)?,
        PixelIntensity::AboveRange => {
            bitwrite.write_bit(false)?;
            bitwrite.write_bit(true)?;
        }
        PixelIntensity::BelowRange => {
            bitwrite.write_bit(false)?;
            bitwrite.write_bit(false)?;
        }
    }
    Ok(())
}

/// Reads a `PixelIntensity` from the given `BitRead`.
fn decode_intensity<T>(bitread: &mut T) -> io::Result<PixelIntensity>
where
    T: BitRead,
{
    let in_range = bitread.read_bit()?;
    if in_range {
        return Ok(PixelIntensity::InRange);
    }
    let above = bitread.read_bit()?;
    if above {
        return Ok(PixelIntensity::AboveRange);
    }
    Ok(PixelIntensity::BelowRange)
}

#[derive(Copy, Clone)]
struct CodingOptions {
    max_context: u32,
    k_values: &'static [u8],
    periodic_count_scaling: Option<u32>,
}

/// Compresses a channel and writes it to the given `BitWrite`.
///
/// # Panics
///
/// This functions assumes that the `channel` is big enough to hold
/// `width*height` pixels. It will panic if the `channel` is not big enough.
fn compress_channel<W>(
    channel: &[i32],
    width: u32,
    height: u32,
    options: CodingOptions,
    bitwrite: &mut W,
) -> io::Result<()>
where
    W: BitWrite,
{
    let total_size: usize = width.checked_mul(height).unwrap().try_into().unwrap();
    assert!(
        channel.len() >= total_size,
        "The channel is not big enough!"
    );

    // Check for edge-case image dimensions.
    match (width, height) {
        (0, _) | (_, 0) => {
            bitwrite.write_signed(i32::BITS, 0)?;
            bitwrite.write_signed(i32::BITS, 0)?;
            return Ok(());
        }
        (1, 1) => {
            bitwrite.write_signed(i32::BITS, channel[0])?;
            bitwrite.write_signed(i32::BITS, 0)?;
            return Ok(());
        }
        _ => {
            bitwrite.write_signed(i32::BITS, channel[0])?;
            bitwrite.write_signed(i32::BITS, channel[1])?;
        }
    };

    let mut estimator: KEstimator = KEstimator::new(
        options.max_context,
        options.k_values,
        options.periodic_count_scaling,
    );

    // Proceed in raster-scan order.
    for i in 2..total_size {
        let (a, b) = misc::nearest_neighbours(i, width as usize).unwrap();

        let p = channel[i];
        let v1 = channel[a];
        let v2 = channel[b];

        let h = cmp::max(v1, v2);
        let l = cmp::min(v1, v2);
        let context: u32 = (h - l).try_into().unwrap();
        let k = estimator.get_k(context);
        let rice_coder = RiceCoder::new(k);

        if p >= l && p <= h {
            encode_intensity(bitwrite, PixelIntensity::InRange)?;
            let to_encode: u32 = (p - l).try_into().unwrap();
            let phase_in_coder = PhaseInCoder::new(context + 1);
            phase_in_coder.encode(bitwrite, to_encode)?;
        } else if p < l {
            encode_intensity(bitwrite, PixelIntensity::BelowRange)?;
            let to_encode: u32 = (l - p - 1).try_into().unwrap();
            rice_coder.encode(bitwrite, to_encode)?;
            estimator.update(context, to_encode);
        } else {
            encode_intensity(bitwrite, PixelIntensity::AboveRange)?;
            let to_encode: u32 = (p - h - 1).try_into().unwrap();
            rice_coder.encode(bitwrite, to_encode)?;
            estimator.update(context, to_encode);
        }
    }
    Ok(())
}

/// Decompresses a channel by reading from the given `BitRead`.
fn decompress_channel<R>(
    width: u32,
    height: u32,
    options: CodingOptions,
    bitread: &mut R,
) -> Result<Vec<i32>, DecompressionError>
where
    R: BitRead,
{
    // Parse the first two pixels.
    let pixel1: i32 = bitread.read_signed(i32::BITS)?;
    let pixel2: i32 = bitread.read_signed(i32::BITS)?;

    // Handle edge-case dimensions.
    match (width, height) {
        (0, _) | (_, 0) => {
            return Ok(vec![]);
        }
        (1, 1) => {
            return Ok(vec![pixel1]);
        }
        _ => (),
    };

    // Create the pixel buffer.
    let total_size: usize = width
        .checked_mul(height)
        .ok_or(DecompressionError::InvalidDimensions)?
        .try_into()
        .map_err(|_| DecompressionError::InvalidDimensions)?;

    let mut buf = vec![0; total_size];
    buf[0] = pixel1;
    buf[1] = pixel2;

    let mut estimator: KEstimator = KEstimator::new(
        options.max_context,
        options.k_values,
        options.periodic_count_scaling,
    );

    // Proceed in raster-scan order.
    for i in 2..total_size {
        let (a, b) = misc::nearest_neighbours(i, width as usize).unwrap();

        let v1 = buf[a];
        let v2 = buf[b];

        let h = cmp::max(v1, v2);
        let l = cmp::min(v1, v2);
        let context: u32 = (h - l).try_into().unwrap();
        let k = estimator.get_k(context);
        let rice_coder = RiceCoder::new(k);

        let intensity = decode_intensity(bitread)?;

        let pixel_value = match intensity {
            PixelIntensity::InRange => {
                let phase_in_coder = PhaseInCoder::new(context + 1);
                let p: i32 = phase_in_coder
                    .decode(bitread)?
                    .try_into()
                    .map_err(|_| DecompressionError::InvalidValue)?;
                p.checked_add(l).ok_or(DecompressionError::ValueOverflow)?
            }
            PixelIntensity::BelowRange => {
                let encoded: u32 = rice_coder.decode(bitread)?;
                estimator.update(context, encoded);
                let encoded: i32 = encoded
                    .try_into()
                    .map_err(|_| DecompressionError::InvalidValue)?;

                // The encoded value is l-p-1.
                // To get p back, we must compute: l-encoded-1.
                l.checked_sub(encoded)
                    .ok_or(DecompressionError::ValueOverflow)?
                    .checked_sub(1)
                    .ok_or(DecompressionError::ValueOverflow)?
            }
            PixelIntensity::AboveRange => {
                let encoded: u32 = rice_coder.decode(bitread)?;
                estimator.update(context, encoded);
                let encoded: i32 = encoded
                    .try_into()
                    .map_err(|_| DecompressionError::InvalidValue)?;
                // The encoded value is p-h-1.
                // To get p back, we must compute: encoded + h + 1.
                encoded
                    .checked_add(h)
                    .ok_or(DecompressionError::ValueOverflow)?
                    .checked_add(1)
                    .ok_or(DecompressionError::ValueOverflow)?
            }
        };
        buf[i] = pixel_value;
    }
    Ok(buf)
}

impl<T> CompressDecompress for ImageBuffer<Luma<T>, Vec<T>>
where
    Luma<T>: Pixel<Subpixel = T>,
    T: Intensity,
{
    fn compress<W>(&self, mut to: W) -> io::Result<()>
    where
        W: Write,
    {
        let (width, height) = self.dimensions();
        write_header(
            Header {
                color_type: ColorType::Gray,
                pixel_depth: T::PIXEL_DEPTH,
                width,
                height,
            },
            &mut to,
        )?;

        let mut bitwriter: BitWriter<W, BigEndian> = BitWriter::new(to);
        let options = CodingOptions {
            max_context: T::MAX_CONTEXT,
            k_values: T::K_VALUES,
            periodic_count_scaling: T::COUNT_SCALING,
        };
        let channel: Vec<i32> = self.as_raw().iter().map(|&x| x.into()).collect();

        compress_channel(&channel, width, height, options, &mut bitwriter)?;
        bitwriter.byte_align()?;
        bitwriter.flush()?;
        Ok(())
    }

    fn decompress_with_header<R>(from: R, header: &Header) -> Result<Self, DecompressionError>
    where
        Self: Sized,
        R: Read,
    {
        if header.color_type != ColorType::Gray {
            return Err(DecompressionError::InvalidColorType);
        }
        if header.pixel_depth != T::PIXEL_DEPTH {
            return Err(DecompressionError::InvalidPixelDepth);
        }

        let mut bitreader: BitReader<R, BigEndian> = BitReader::new(from);
        let options = CodingOptions {
            max_context: T::MAX_CONTEXT,
            k_values: T::K_VALUES,
            periodic_count_scaling: T::COUNT_SCALING,
        };
        let channel = decompress_channel(header.width, header.height, options, &mut bitreader)?;

        // Channel is Vec<i32>, convert back to T.
        let mut result: Vec<T> = vec![T::default(); channel.len()];
        for (i, &value) in channel.iter().enumerate() {
            result[i] = value
                .try_into()
                .map_err(|_| DecompressionError::InvalidValue)?;
        }

        let image = ImageBuffer::from_raw(header.width, header.height, result).unwrap();
        Ok(image)
    }
}

impl<T> CompressDecompress for ImageBuffer<Rgb<T>, Vec<T>>
where
    Rgb<T>: Pixel<Subpixel = T>,
    T: Intensity,
{
    fn compress<W>(&self, mut to: W) -> io::Result<()>
    where
        W: Write,
    {
        let (width, height) = self.dimensions();
        write_header(
            Header {
                color_type: ColorType::Rgb,
                pixel_depth: T::PIXEL_DEPTH,
                width,
                height,
            },
            &mut to,
        )?;

        let num_pixels = (width as usize) * (height as usize);
        let pixels = self.as_raw();

        let (mut y, mut co, mut cg) = (
            vec![0; num_pixels],
            vec![0; num_pixels],
            vec![0; num_pixels],
        );

        for i in 0..num_pixels {
            let current = i * 3;
            let (ly, lco, lcg) = rgb_to_ycocg(
                pixels[current].into(),
                pixels[current + 1].into(),
                pixels[current + 2].into(),
            );
            y[i] = ly;
            co[i] = lco;
            cg[i] = lcg;
        }

        let mut bitwriter: BitWriter<W, BigEndian> = BitWriter::new(to);
        let options = CodingOptions {
            max_context: T::MAX_CONTEXT,
            k_values: T::K_VALUES,
            periodic_count_scaling: T::COUNT_SCALING,
        };

        compress_channel(&y, width, height, options, &mut bitwriter)?;
        compress_channel(&co, width, height, options, &mut bitwriter)?;
        compress_channel(&cg, width, height, options, &mut bitwriter)?;
        bitwriter.byte_align()?;
        bitwriter.flush()?;
        Ok(())
    }

    fn decompress_with_header<R>(from: R, header: &Header) -> Result<Self, DecompressionError>
    where
        Self: Sized,
        R: Read,
    {
        if header.color_type != ColorType::Rgb {
            return Err(DecompressionError::InvalidColorType);
        }
        if header.pixel_depth != T::PIXEL_DEPTH {
            return Err(DecompressionError::InvalidPixelDepth);
        }

        let mut bitreader: BitReader<R, BigEndian> = BitReader::new(from);
        let options = CodingOptions {
            max_context: T::MAX_CONTEXT,
            k_values: T::K_VALUES,
            periodic_count_scaling: T::COUNT_SCALING,
        };

        let y = decompress_channel(header.width, header.height, options, &mut bitreader)?;
        let co = decompress_channel(header.width, header.height, options, &mut bitreader)?;
        let cg = decompress_channel(header.width, header.height, options, &mut bitreader)?;

        let num_pixels = (header.width as usize) * (header.height as usize);
        let buf_size = num_pixels
            .checked_mul(Rgb::CHANNEL_COUNT as usize)
            .ok_or(DecompressionError::InvalidDimensions)?;

        let mut buf = vec![T::default(); buf_size];
        for i in 0..num_pixels {
            let (r, g, b) = ycocg_to_rgb(y[i], co[i], cg[i]);
            buf[i * 3] = r.try_into().map_err(|_| DecompressionError::InvalidValue)?;
            buf[i * 3 + 1] = g.try_into().map_err(|_| DecompressionError::InvalidValue)?;
            buf[i * 3 + 2] = b.try_into().map_err(|_| DecompressionError::InvalidValue)?;
        }
        Ok(ImageBuffer::from_raw(header.width, header.height, buf).unwrap())
    }
}

pub fn compress_image<W, T>(to: W, image: T) -> io::Result<()>
where
    W: Write,
    T: CompressDecompress,
{
    image.compress(to)
}

pub fn decompress_image<R>(mut from: R) -> Result<DynamicImage, DecompressionError>
where
    R: Read,
{
    let header = read_header(&mut from)?;

    let result = match (&header.color_type, &header.pixel_depth) {
        (ColorType::Gray, PixelDepth::Eight) => {
            DynamicImage::ImageLuma8(CompressDecompress::decompress_with_header(from, &header)?)
        }
        (ColorType::Gray, PixelDepth::Sixteen) => {
            DynamicImage::ImageLuma16(CompressDecompress::decompress_with_header(from, &header)?)
        }
        (ColorType::Rgb, PixelDepth::Eight) => {
            DynamicImage::ImageRgb8(CompressDecompress::decompress_with_header(from, &header)?)
        }
        (ColorType::Rgb, PixelDepth::Sixteen) => {
            DynamicImage::ImageRgb16(CompressDecompress::decompress_with_header(from, &header)?)
        }
    };
    Ok(result)
}

#[cfg(test)]
mod test {
    use super::{CompressDecompress, Pixel};
    use image::{GrayImage, ImageBuffer, Luma, Rgb};
    use rand::{
        self,
        distributions::{Distribution, Standard},
        rngs::ThreadRng,
        Rng,
    };
    use std::fmt::Debug;
    use std::io::Cursor;

    #[test]
    fn test_compression_zero_width() {
        let image = GrayImage::new(0, 3);
        let mut sink = Vec::new();
        image.compress(&mut sink).unwrap();
        let decompressed = GrayImage::decompress(&mut Cursor::new(sink)).unwrap();
        assert_eq!(image, decompressed);
    }

    // Returns a random image with the given dimensions.
    fn random_grayscale<T>(
        width: u32,
        height: u32,
        rng: &mut ThreadRng,
    ) -> ImageBuffer<Luma<T>, Vec<T>>
    where
        Luma<T>: Pixel<Subpixel = T>,
        Standard: Distribution<T>,
    {
        let mut image = ImageBuffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let pixel_intensity: T = rng.gen();
                image.put_pixel(x, y, Luma([pixel_intensity]));
            }
        }
        image
    }

    fn random_rgb<T>(width: u32, height: u32, rng: &mut ThreadRng) -> ImageBuffer<Rgb<T>, Vec<T>>
    where
        Rgb<T>: Pixel<Subpixel = T>,
        Standard: Distribution<T>,
    {
        let mut image = ImageBuffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let (a, b, c) = (rng.gen(), rng.gen(), rng.gen());
                image.put_pixel(x, y, Rgb([a, b, c]));
            }
        }
        image
    }

    #[test]
    fn test_compression_decompression_grayscale() {
        let dimensions = vec![
            (2, 1),
            (1, 2),
            (1, 1),
            (4, 7),
            (100, 40),
            (124, 274),
            (1447, 8),
            (44, 1),
            (1, 100),
            (680, 480),
        ];
        let mut rng = rand::thread_rng();

        for (width, height) in dimensions {
            let image = random_grayscale::<u8>(width, height, &mut rng);

            let mut sink = Vec::new();
            image.compress(&mut sink).unwrap();
            let decompressed = CompressDecompress::decompress(&mut Cursor::new(sink)).unwrap();
            assert_eq!(image, decompressed);

            let image = random_grayscale::<u16>(width, height, &mut rng);
            let mut sink = Vec::new();
            image.compress(&mut sink).unwrap();
            let decompressed = CompressDecompress::decompress(&mut Cursor::new(sink)).unwrap();
            assert_eq!(image, decompressed);
        }
    }

    // Compresses an image and then decompresses it to check if
    // decompress(compress(x)) = x
    fn compress_then_decompress<T>(image: T)
    where
        T: CompressDecompress + Eq + Debug,
    {
        let mut sink = Vec::new();
        image.compress(&mut sink).unwrap();
        let decompressed = CompressDecompress::decompress(Cursor::new(sink)).unwrap();
        assert_eq!(image, decompressed);
    }

    #[test]
    #[ignore]
    fn test_compression_decompression_intensive() {
        let mut rng = rand::thread_rng();

        for width in 0..20 {
            for height in 0..20 {
                compress_then_decompress(random_grayscale::<u8>(width, height, &mut rng));
                compress_then_decompress(random_grayscale::<u16>(width, height, &mut rng));

                compress_then_decompress(random_rgb::<u8>(width, height, &mut rng));
                compress_then_decompress(random_rgb::<u16>(width, height, &mut rng));
            }
        }
    }
}
