use image::{GenericImageView, GrayImage, Luma};

/// A grayscale image that was compressed using the felics algorithm.
pub struct CompressedGrayscaleImage {
    width: u32,
    height: u32,
    // We need to store the first two pixels unencoded.
    pixel1: Luma<u8>,
    pixel2: Luma<u8>,
}

#[derive(Debug)]
/// Errors that may occur when compressing an image.
pub enum CompressionError {
    ImageTooSmall,
}

impl TryFrom<GrayImage> for CompressedGrayscaleImage {
    type Error = CompressionError;

    fn try_from(image: GrayImage) -> Result<Self, Self::Error> {
        let mut pixels = image.enumerate_pixels();

        let pixel1 = match pixels.next() {
            Some((_, _, &luma)) => luma,
            None => return Err(CompressionError::ImageTooSmall),
        };

        let pixel2 = match pixels.next() {
            Some((_, _, &luma)) => luma,
            None => return Err(CompressionError::ImageTooSmall),
        };

        Ok(CompressedGrayscaleImage {
            width: image.width(),
            height: image.height(),
            pixel1,
            pixel2,
        })
    }
}

/// Returns the two nearest neighbours of a pixel in a given image, that have already been coded.
///
/// Except along the top and left edges, these are the pixel above and the pixel
/// to the left of the pixel.
fn nearest_neighbours<T>((x, y): (u32, u32), img: &T) -> Option<((u32, u32), (u32, u32))>
where
    T: GenericImageView,
{
    assert!(img.in_bounds(x, y));
    if x > 0 && y > 0 {
        Some(((x - 1, y), (x, y - 1)))
    } else if y == 0 {
        if x >= 2 {
            Some(((x - 1, y), (x - 2, y)))
        } else {
            None
        }
    } else {
        if y >= 2 {
            Some(((x, y - 1), (x, y - 2)))
        } else if img.in_bounds(x, y - 1) && img.in_bounds(x + 1, y - 1) {
            Some(((x, y - 1), (x + 1, y - 1)))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::nearest_neighbours;
    use image::{GenericImageView, Luma};

    struct ImageMock {
        width: u32,
        height: u32,
    }

    impl ImageMock {
        fn new(width: u32, height: u32) -> ImageMock {
            assert_ne!(width, 0);
            assert_ne!(height, 0);
            ImageMock { width, height }
        }
    }

    impl GenericImageView for ImageMock {
        type Pixel = Luma<u8>;

        fn dimensions(&self) -> (u32, u32) {
            (self.width, self.height)
        }

        fn bounds(&self) -> (u32, u32, u32, u32) {
            (0, 0, self.width, self.height)
        }
        fn get_pixel(&self, _: u32, _: u32) -> Self::Pixel {
            Luma([0])
        }
    }

    #[test]
    fn test_nearest_neighbours() {
        let image = ImageMock::new(23, 25);
        assert_eq!(nearest_neighbours((5, 8), &image), Some(((4, 8), (5, 7))));
        assert_eq!(nearest_neighbours((0, 8), &image), Some(((0, 7), (0, 6))));
        assert_eq!(nearest_neighbours((2, 0), &image), Some(((1, 0), (0, 0))));
        assert_eq!(nearest_neighbours((1, 1), &image), Some(((0, 1), (1, 0))));
        assert_eq!(nearest_neighbours((1, 0), &image), None);
        assert_eq!(nearest_neighbours((0, 1), &image), Some(((0, 0), (1, 0))));

        let image = ImageMock::new(5, 1);
        assert_eq!(nearest_neighbours((1, 0), &image), None);
        assert_eq!(nearest_neighbours((2, 0), &image), Some(((1, 0), (0, 0))));
        assert_eq!(nearest_neighbours((4, 0), &image), Some(((3, 0), (2, 0))));

        let image = ImageMock::new(1, 30);
        assert_eq!(nearest_neighbours((0, 0), &image), None);
        assert_eq!(nearest_neighbours((0, 1), &image), None);
        assert_eq!(nearest_neighbours((0, 2), &image), Some(((0, 1), (0, 0))));
        assert_eq!(nearest_neighbours((0, 10), &image), Some(((0, 9), (0, 8))));

        let image = ImageMock::new(2, 2);
        assert_eq!(nearest_neighbours((0, 1), &image), Some(((0, 0), (1, 0))));
        assert_eq!(nearest_neighbours((1, 1), &image), Some(((0, 1), (1, 0))));
    }

    #[test]
    #[should_panic]
    fn test_nearest_neighbours_out_of_bounds() {
        let image = ImageMock::new(100, 80);
        nearest_neighbours((90, 80), &image);
    }
}
