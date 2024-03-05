use felics::compression::CompressDecompress;
use image::{self, DynamicImage};
use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

struct BenchmarkMetrics {
    compress_tm: f64,
    decompress_tm: f64,
    compress_size: usize,
}

fn compress_image<T>(im: T) -> BenchmarkMetrics
where
    T: CompressDecompress + Debug + Eq,
{
    let now = Instant::now();
    let compressed = im.compress();
    let compress_tm = now.elapsed().as_secs_f64();

    let now = Instant::now();
    let decompressed = CompressDecompress::decompress(&compressed).unwrap();
    let decompress_tm = now.elapsed().as_secs_f64();

    assert_eq!(im, decompressed);

    BenchmarkMetrics {
        compress_tm,
        decompress_tm,
        compress_size: compressed.size(),
    }
}

fn compress_file(path: &PathBuf) -> BenchmarkMetrics {
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let image = image::open(&path).unwrap();

    let (width, height) = (image.width(), image.height());
    let name;

    let results = match image {
        DynamicImage::ImageLuma8(image) => {
            name = format!("{}-{}x{}-8-bit grayscale", file_name, width, height);
            compress_image(image)
        }
        DynamicImage::ImageLuma16(image) => {
            name = format!("{}-{}x{}-16-bit grayscale", file_name, width, height);
            compress_image(image)
        }
        DynamicImage::ImageRgb8(image) => {
            name = format!("{}-{}x{}-8-bit rgb", file_name, width, height);
            compress_image(image)
        }
        DynamicImage::ImageRgb16(image) => {
            name = format!("{}-{}x{}-16-bit rgb", file_name, width, height);
            compress_image(image)
        }
        _ => panic!("Unknown format!"),
    };

    println!(
        "{} - CTime: {}, DTime: {}, Size: {}",
        name, results.compress_tm, results.decompress_tm, results.compress_size
    );
    results
}

/// Compress all the images in the test suite and report
/// various metrics.
#[test]
fn compress_suite() {
    let folders = vec![
        format!("{}/image-suite/grayscale/8bit", env!("CARGO_MANIFEST_DIR")),
        format!("{}/image-suite/grayscale/16bit", env!("CARGO_MANIFEST_DIR")),
        format!("{}/image-suite/rgb/8bit", env!("CARGO_MANIFEST_DIR")),
    ];

    for folder in folders {
        println!("Entering folder: {}", folder);
        let files = fs::read_dir(folder).unwrap();

        let mut total_compress_tm = 0.0;
        let mut total_decompress_tm = 0.0;
        let mut total_size = 0;

        for file in files {
            let entry_path = file.unwrap().path();
            let metrics = compress_file(&entry_path);

            total_compress_tm += metrics.compress_tm;
            total_decompress_tm += metrics.decompress_tm;
            total_size += metrics.compress_size;
        }

        println!(
            "Total - CTime: {}, DTime: {}, Size: {}",
            total_compress_tm, total_decompress_tm, total_size
        );
    }
}
