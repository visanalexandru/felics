use image::GenericImageView;
use std::iter::Iterator;

/// An iterator that enumerates pixel positions in their raster-scan order.
pub struct RasterScan {
    image_width: u32,
    image_height: u32,
    x: u32,
    y: u32,
}

impl RasterScan {
    /// Returns a new `RasterScan` iterator that will enumerate pixels
    /// in their raster-scan order in an image of dimensions `(width, height)`.
    ///
    /// Panics if the width or height is 0.
    pub fn new(width: u32, height: u32) -> RasterScan {
        assert!(width > 0 && height > 0);
        RasterScan {
            image_width: width,
            image_height: height,
            x: 0,
            y: 0,
        }
    }
}

impl Iterator for RasterScan {
    type Item = (u32, u32);
    fn next(&mut self) -> Option<Self::Item> {
        if self.y >= self.image_height {
            return None;
        }

        let current = (self.x, self.y);

        self.x += 1;
        if self.x == self.image_width {
            self.x = 0;
            self.y += 1;
        }

        Some(current)
    }
}

/// Returns the two nearest neighbours of a pixel in a given image, that have already been coded.
///
/// Except along the top and left edges, these are the pixel above and the pixel
/// to the left of the pixel.
pub fn nearest_neighbours<T>((x, y): (u32, u32), img: &T) -> Option<((u32, u32), (u32, u32))>
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
    use super::{nearest_neighbours, RasterScan};
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

    fn get_raster_scan(width: u32, height: u32) -> Vec<(u32, u32)> {
        let mut result = Vec::new();
        for y in 0..height {
            for x in 0..width {
                result.push((x, y));
            }
        }
        result
    }

    #[test]
    #[should_panic]
    fn test_raster_scan_empty_image() {
        let _ = RasterScan::new(0, 0);
    }

    #[test]
    fn test_raster_scan() {
        let iter = RasterScan::new(4, 2);
        let pixels: Vec<(u32, u32)> = iter.collect();
        assert_eq!(pixels, get_raster_scan(4, 2));

        let iter = RasterScan::new(1, 80);
        let pixels: Vec<(u32, u32)> = iter.collect();
        assert_eq!(pixels, get_raster_scan(1, 80));

        let iter = RasterScan::new(30, 1);
        let pixels: Vec<(u32, u32)> = iter.collect();
        assert_eq!(pixels, get_raster_scan(30, 1));

        let iter = RasterScan::new(1, 1);
        let pixels: Vec<(u32, u32)> = iter.collect();
        assert_eq!(pixels, get_raster_scan(1, 1));

        let iter = RasterScan::new(14, 6);
        let pixels: Vec<(u32, u32)> = iter.collect();
        assert_eq!(pixels, get_raster_scan(14, 6));
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
