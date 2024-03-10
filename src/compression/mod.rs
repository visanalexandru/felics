use crate::coding::{phase_in_coding::PhaseInCoder, rice_coding::RiceCoder};
use bitstream_io::{self, BigEndian, BitRead, BitReader, BitWrite, BitWriter};
use error::DecompressionError;
pub use format::{read_header, write_header, ColorType, Header, PixelDepth};
use image::{ImageBuffer, Luma, Pixel, Rgb};
use parameter_selection::KEstimator;
use std::cmp;
use std::io::{self, Read, Write};
pub use traits::{CompressDecompress, Intensity};

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

/// Compresses a channel and writes it to the given `BitWrite`.
///
/// # Panics
///
/// This functions assumes that the `channel` is big enough to hold
/// `width*height` pixels. It will panic if the `channel` is not big enough.
fn compress_channel<T, W>(
    channel: &[T],
    width: u32,
    height: u32,
    bitwrite: &mut W,
) -> io::Result<()>
where
    T: Intensity,
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
            T::zero().write(bitwrite)?;
            T::zero().write(bitwrite)?;
            return Ok(());
        }
        (1, 1) => {
            channel[0].write(bitwrite)?;
            T::zero().write(bitwrite)?;
            return Ok(());
        }
        _ => {
            channel[0].write(bitwrite)?;
            channel[1].write(bitwrite)?;
        }
    };

    let mut estimator: KEstimator<T> = KEstimator::new(true);
    // Proceed in raster-scan order.
    for i in 2..total_size {
        let (a, b) = misc::nearest_neighbours(i, width as usize).unwrap();

        let p = channel[i];
        let v1 = channel[a];
        let v2 = channel[b];

        let h = cmp::max(v1, v2);
        let l = cmp::min(v1, v2);
        let context = h - l;
        let k = estimator.get_k(context);
        let rice_coder = RiceCoder::new(k);

        if p >= l && p <= h {
            encode_intensity(bitwrite, PixelIntensity::InRange)?;
            let to_encode = p - l;
            let phase_in_coder = PhaseInCoder::new(Into::<u32>::into(context) + 1);
            phase_in_coder.encode(bitwrite, to_encode.into())?;
        } else if p < l {
            encode_intensity(bitwrite, PixelIntensity::BelowRange)?;
            let to_encode = l - p - T::one();
            rice_coder.encode(bitwrite, to_encode.into())?;
            estimator.update(context, to_encode);
        } else {
            encode_intensity(bitwrite, PixelIntensity::AboveRange)?;
            let to_encode = p - h - T::one();
            rice_coder.encode(bitwrite, to_encode.into())?;
            estimator.update(context, to_encode);
        }
    }
    Ok(())
}

/// Decompresses a channel by reading from the given `BitRead`.
fn decompress_channel<T, R>(
    width: u32,
    height: u32,
    bitread: &mut R,
) -> Result<Vec<T>, DecompressionError>
where
    T: Intensity,
    R: BitRead,
{
    // Parse the first two pixels.
    let pixel1: T = T::read(bitread)?;
    let pixel2: T = T::read(bitread)?;

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

    let mut buf = vec![T::zero(); total_size];
    buf[0] = pixel1;
    buf[1] = pixel2;

    let mut estimator: KEstimator<T> = KEstimator::new(true);

    // Proceed in raster-scan order.
    for i in 2..total_size {
        let (a, b) = misc::nearest_neighbours(i, width as usize).unwrap();

        let v1 = buf[a];
        let v2 = buf[b];

        let h = cmp::max(v1, v2);
        let l = cmp::min(v1, v2);
        let context = h - l;
        let k = estimator.get_k(context);
        let rice_coder = RiceCoder::new(k);

        let intensity = decode_intensity(bitread)?;

        let pixel_value = match intensity {
            PixelIntensity::InRange => {
                let phase_in_coder = PhaseInCoder::new(Into::<u32>::into(context) + 1);
                let p: T = phase_in_coder
                    .decode(bitread)?
                    .try_into()
                    .map_err(|_| DecompressionError::InvalidValue)?;
                p.checked_add(&l).ok_or(DecompressionError::ValueOverflow)?
            }
            PixelIntensity::BelowRange => {
                let encoded: T = rice_coder
                    .decode(bitread)?
                    .try_into()
                    .map_err(|_| DecompressionError::InvalidValue)?;
                estimator.update(context, encoded);
                // The encoded value is l-p-1.
                // To get p back, we must compute: l-encoded-1.
                l.checked_sub(&encoded)
                    .ok_or(DecompressionError::ValueOverflow)?
                    .checked_sub(&T::one())
                    .ok_or(DecompressionError::ValueOverflow)?
            }
            PixelIntensity::AboveRange => {
                let encoded: T = rice_coder
                    .decode(bitread)?
                    .try_into()
                    .map_err(|_| DecompressionError::InvalidValue)?;
                estimator.update(context, encoded);
                // The encoded value is p-h-1.
                // To get p back, we must compute: encoded + h + 1.
                encoded
                    .checked_add(&h)
                    .ok_or(DecompressionError::ValueOverflow)?
                    .checked_add(&T::one())
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
        compress_channel(self.as_raw(), width, height, &mut bitwriter)?;
        bitwriter.byte_align()?;
        bitwriter.flush()?;
        Ok(())
    }

    fn decompress<R>(mut from: R) -> Result<Self, DecompressionError>
    where
        Self: Sized,
        R: Read,
    {
        let header = read_header(&mut from)?;
        if header.color_type != ColorType::Gray {
            return Err(DecompressionError::InvalidColorType);
        }
        if header.pixel_depth != T::PIXEL_DEPTH {
            return Err(DecompressionError::InvalidPixelDepth);
        }
        let mut bitreader: BitReader<R, BigEndian> = BitReader::new(from);
        let channel = decompress_channel(header.width, header.height, &mut bitreader)?;
        let image = ImageBuffer::from_raw(header.width, header.height, channel).unwrap();
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

        let (mut red, mut green, mut blue) = (
            vec![T::zero(); num_pixels],
            vec![T::zero(); num_pixels],
            vec![T::zero(); num_pixels],
        );

        for i in 0..num_pixels {
            let current = i * 3;
            red[i] = pixels[current];
            green[i] = pixels[current + 1];
            blue[i] = pixels[current + 2];
        }

        let mut bitwriter: BitWriter<W, BigEndian> = BitWriter::new(to);
        compress_channel(&red, width, height, &mut bitwriter)?;
        compress_channel(&green, width, height, &mut bitwriter)?;
        compress_channel(&blue, width, height, &mut bitwriter)?;
        bitwriter.byte_align()?;
        bitwriter.flush()?;
        Ok(())
    }

    fn decompress<R>(mut from: R) -> Result<Self, DecompressionError>
    where
        Self: Sized,
        R: Read,
    {
        let header = read_header(&mut from)?;
        if header.color_type != ColorType::Rgb {
            return Err(DecompressionError::InvalidColorType);
        }
        if header.pixel_depth != T::PIXEL_DEPTH {
            return Err(DecompressionError::InvalidPixelDepth);
        }

        let mut bitreader: BitReader<R, BigEndian> = BitReader::new(from);
        let red = decompress_channel(header.width, header.height, &mut bitreader)?;
        let green = decompress_channel(header.width, header.height, &mut bitreader)?;
        let blue = decompress_channel(header.width, header.height, &mut bitreader)?;

        let num_pixels = (header.width as usize) * (header.height as usize);
        let buf_size = num_pixels
            .checked_mul(Rgb::CHANNEL_COUNT as usize)
            .ok_or(DecompressionError::InvalidDimensions)?;

        let mut buf = vec![T::zero(); buf_size];
        for i in 0..num_pixels {
            buf[i * 3] = red[i];
            buf[i * 3 + 1] = green[i];
            buf[i * 3 + 2] = blue[i];
        }
        Ok(ImageBuffer::from_raw(header.width, header.height, buf).unwrap())
    }
}

#[cfg(test)]
mod test {
    use super::{CompressDecompress, Pixel};
    use image::{GrayImage, ImageBuffer, Luma};
    use rand::{
        self,
        distributions::{Distribution, Standard},
        rngs::ThreadRng,
        Rng,
    };
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

    #[test]
    #[ignore]
    fn test_compression_decompression_intensive() {
        let mut rng = rand::thread_rng();

        for width in 0..100 {
            for height in 0..100 {
                println!("{} {}", width, height);
                let image = random_grayscale::<u8>(width, height, &mut rng);

                let mut sink = Vec::new();
                image.compress(&mut sink).unwrap();
                let decompressed = CompressDecompress::decompress(&mut Cursor::new(sink)).unwrap();
                assert_eq!(image, decompressed);

                let mut sink = Vec::new();
                let image = random_grayscale::<u16>(width, height, &mut rng);
                image.compress(&mut sink).unwrap();
                let decompressed = CompressDecompress::decompress(&mut Cursor::new(sink)).unwrap();
                assert_eq!(image, decompressed);
            }
        }
    }
}
