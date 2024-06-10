use clap::Parser;
use felics::compression::decompress_image;
use show_image::*;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::process;

#[derive(Parser, Debug)]
#[command(about = "Visualizes a felics file", long_about = None)]
#[command(version)]
struct Args {
    /// The path to the felics file.
    input: PathBuf,
}

#[show_image::main]
fn main() {
    let args = Args::parse();

    let input_file = match File::open(&args.input) {
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

    let filename = args.input.file_name().unwrap().to_str().unwrap();

    let window = match create_window(filename, Default::default()) {
        Err(e) => {
            println!("Cannot create window: {}", e);
            process::exit(1);
        }
        Ok(w) => w,
    };

    if let Err(e) = window.set_image(filename, dyn_image) {
        println!("Cannot show image: {}", e);
        process::exit(1);
    }

    let channel = window.event_channel().unwrap();
    for event in channel {
        if let event::WindowEvent::KeyboardInput(event) = event {
            if event.input.key_code == Some(event::VirtualKeyCode::Escape)
                && event.input.state.is_pressed()
            {
                break;
            }
        }
    }
}
