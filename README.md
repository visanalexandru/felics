# FELICS 

FELICS, short for "Fast and Efficient Lossless Image Compression", is an image compression method 
that gives compression comparable to JPEG's lossless mode with about five times the speed. 

You can read the original paper [here](https://www.researchgate.net/publication/2773317_Fast_and_Efficient_Lossless_Image_Compression).

## Running the tests

To run the tests while ignoring the expensive tests:

`cargo test`

To run all the tests:

`cargo test -- --include-ignored`