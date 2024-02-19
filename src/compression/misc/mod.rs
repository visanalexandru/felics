/// Returns the two nearest neighbours of a pixel in a given image, that have already been visited
/// in a raster scan.
///
/// Except along the top and left edges, these are the pixel above and the pixel
/// to the left of the pixel.
pub fn nearest_neighbours(i: usize, width: usize) -> Option<(usize, usize)> {
    let (x, y) = (i % width, i / width);

    if x > 0 && y > 0 {
        Some((i - 1, i - width))
    } else if y == 0 {
        if x >= 2 {
            Some((i - 1, i - 2))
        } else {
            None
        }
    } else if y >= 2 {
        Some((i - width, i - 2 * width))
    } else if (x + 1) < width {
        Some((i - width, i - width + 1))
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::nearest_neighbours;
    pub fn pti((x, y): (usize, usize), width: usize) -> usize {
        return y * width + x;
    }
    #[test]
    fn test_nearest_neighbours() {
        let width = 23;

        assert_eq!(
            nearest_neighbours(pti((5, 8), width), width),
            Some((pti((4, 8), width), pti((5, 7), width)))
        );

        assert_eq!(
            nearest_neighbours(pti((0, 8), width), width),
            Some((pti((0, 7), width), pti((0, 6), width)))
        );

        assert_eq!(
            nearest_neighbours(pti((2, 0), width), width),
            Some((pti((1, 0), width), pti((0, 0), width)))
        );

        assert_eq!(
            nearest_neighbours(pti((1, 1), width), width),
            Some((pti((0, 1), width), pti((1, 0), width)))
        );
        assert_eq!(nearest_neighbours(pti((1, 0), width), width), None);

        assert_eq!(
            nearest_neighbours(pti((0, 1), width), width),
            Some((pti((0, 0), width), pti((1, 0), width)))
        );

        let width = 1;
        assert_eq!(nearest_neighbours(pti((0, 0), width), width), None);
        assert_eq!(nearest_neighbours(pti((0, 1), width), width), None);
        assert_eq!(
            nearest_neighbours(pti((0, 2), width), width),
            Some((pti((0, 1), width), pti((0, 0), width)))
        );
    }
}
