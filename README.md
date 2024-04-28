# FELICS 

FELICS, short for "Fast and Efficient Lossless Image Compression", is an image compression method 
that gives compression comparable to JPEG's lossless mode with about five times the speed. 

You can read the original paper [here](https://www.researchgate.net/publication/2773317_Fast_and_Efficient_Lossless_Image_Compression).

## Running the tests

To run the tests while ignoring the expensive tests:

`cargo test`

To run all the tests:

`cargo test -- --include-ignored`


## Building and installing

To build the project:

`cargo build`

To install `cfelics` and `dfelics` (tools to convert to/from other image formats):

`cargo install --path .`


## Running the benchmarks

First, you need to install the following dependencies:

- ImageMagick's [convert](https://imagemagick.org/script/download.php)
- [cwebp](https://developers.google.com/speed/webp/download) and `dwebp`
- `cfelics` and `dfelics`
- python3 (this was tested on version 3.10.12)

To run the benchmarks:

```
cd bench/
python3 benchmark-big-corpus.py
```

