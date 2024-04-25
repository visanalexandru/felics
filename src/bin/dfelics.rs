use clap::Parser;
use felics::compression::decompress_image;
use image::DynamicImage;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::process;

#[derive(Parser, Debug)]
#[command(about = "Decompresses a felics file to another image file", long_about = None)]
#[command(version)]
struct Args {
    /// The input felics file.
    #[arg(short, long)]
    input: PathBuf,

    /// The output file. The output format will be determined using
    /// the extension of the output file.
    #[arg(short, long)]
    output: PathBuf,
}

fn main() {
    let args = Args::parse();

    let input_file = match File::open(args.input) {
        Err(e) => {
            println!("Cannot open input file: {}", e);
            process::exit(1);
        }
        Ok(f) => f,
    };

    let reader = BufReader::new(input_file);

    let dyn_image = match decompress_image(reader) {
        Err(error) => {
            println!("Error while decompressing the image: {:?}", error);
            process::exit(1)
        }
        Ok(d) => d,
    };

    let result = match dyn_image {
        DynamicImage::ImageLuma8(luma8) => luma8.save(args.output),
        DynamicImage::ImageLuma16(luma16) => luma16.save(args.output),
        DynamicImage::ImageRgb8(rgb8) => rgb8.save(args.output),
        DynamicImage::ImageRgb16(rgb16) => rgb16.save(args.output),
        _ => {
            panic!("Unknown format!")
        }
    };

    if let Err(e) = result {
        println!("Cannot save image: {}", e);
        process::exit(1)
    }
}
