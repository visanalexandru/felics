use clap::Parser;
use felics::compression::CompressDecompress;
use image::{self, io::Reader, DynamicImage};
use std::fs::File;
use std::io::{self, BufWriter};
use std::path::PathBuf;
use std::process;

// Use clap to define the argument list.

#[derive(Parser, Debug)]
#[command(about = "Compresses an image file to a felics file", long_about = None)]
#[command(version)]
struct Args {
    /// The input file.
    #[arg(short, long)]
    input: PathBuf,

    /// The output felics file.
    #[arg(short, long)]
    output: PathBuf,
}

fn compress_to<T>(image: T, path: PathBuf) -> io::Result<()>
where
    T: CompressDecompress,
{
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    image.compress(writer)
}

fn main() {
    let args = Args::parse();

    let reader = match Reader::open(args.input) {
        Ok(r) => r,
        Err(e) => {
            println!("Cannot open file: {}", e);
            process::exit(1)
        }
    };

    let dynamic_image = match reader.decode() {
        Ok(d) => d,
        Err(e) => {
            println!("Cannot decode image: {}", e);
            process::exit(1)
        }
    };

    let result = match dynamic_image {
        DynamicImage::ImageLuma8(luma8) => {
            println!("Compressing 8-bit grayscale image...");
            compress_to(luma8, args.output)
        }
        DynamicImage::ImageLuma16(luma16) => {
            println!("Compressing 16-bit grayscale image...");
            compress_to(luma16, args.output)
        }
        DynamicImage::ImageRgb8(rgb8) => {
            println!("Compressing 8-bit rgb image...");
            compress_to(rgb8, args.output)
        }
        DynamicImage::ImageRgb16(rgb16) => {
            println!("Compressing 16-bit rgb image...");
            compress_to(rgb16, args.output)
        }
        _ => {
            println!("Unsupported image format: {:?}", dynamic_image.color());
            process::exit(1)
        }
    };

    if let Err(e) = result {
        println!("Cannot compress image: {e}");
        process::exit(1)
    }
}
