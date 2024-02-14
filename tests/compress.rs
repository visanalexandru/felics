use felics::compression::CompressDecompress;
use image::{self, DynamicImage, GrayImage};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

struct BenchmarkMetrics {
    compress_tm: f64,
    decompress_tm: f64,
    compress_size: usize,
}

fn benchmark_file(path: &PathBuf) -> BenchmarkMetrics {
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let image = image::open(&path).unwrap();
    let image = match image {
        DynamicImage::ImageLuma8(image) => image,
        _ => panic!("{:?} is not a grayscale image!", file_name),
    };
    let (width, height) = image.dimensions();

    // Compress, Decompress and report statistics.
    // Also make sure that the image is the same after decompression.
    let now = Instant::now();
    let compressed = image.compress();
    let compress_tm = now.elapsed().as_secs_f64();

    let now = Instant::now();
    let decompressed = GrayImage::decompress(&compressed).unwrap();
    let decompress_tm = now.elapsed().as_secs_f64();

    assert_eq!(image, decompressed);

    println!(
        "{}-{}x{} - CTime: {}, DTime: {}, Size: {}",
        file_name,
        width,
        height,
        compress_tm,
        decompress_tm,
        compressed.size()
    );

    BenchmarkMetrics {
        compress_tm,
        decompress_tm,
        compress_size: compressed.size(),
    }
}

/// Compress all the images in the test suite and report
/// various metrics.
#[test]
fn compress_suite() {
    let root = format!("{}/grayscale-suite", env!("CARGO_MANIFEST_DIR"));
    let files = fs::read_dir(root).unwrap();

    let mut total_compress_tm = 0.0;
    let mut total_decompress_tm = 0.0;
    let mut total_size = 0;

    for file in files {
        let entry_path = file.unwrap().path();
        let metrics = benchmark_file(&entry_path);

        total_compress_tm += metrics.compress_tm;
        total_decompress_tm += metrics.decompress_tm;
        total_size += metrics.compress_size;
    }
    println!(
        "Total - CTime: {}, DTime: {}, Size: {}",
        total_compress_tm, total_decompress_tm, total_size
    );
}
