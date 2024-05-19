# A new lossless image compression format based on FELICS 

## Introduction

In this paper, we will describe a new lossless image compression format that is very simple yet efficient. We plan to use FELICS [6](Howard & Vitter, 2002) as a method of compressing grayscale images. We will then generalize this method to compress RGB images and add support for both 8-bit and 16-bit pixel depths.

In the end, we should have a specification for our new image format, tools to convert from other image formats to ours and backward, and a library that allows users to compress/decompress images from their code.

## Preliminaries

### Data compression 

Data compression is the process of reducing the number of bits required to encode information.
It is particularly useful for reducing storage costs and enables transmission over limited bandwith channels. 
Data compression can be lossless or lossy. 

As the name suggests, lossless compression allows the reconstruction of data from the compressed data without losing any information. Lossless compression is often used when no difference between the original data and the reconstructed data is tolerated [1](Sayood, 2006, p. 4).  

On the other hand, data that has been compressed using lossy compression cannot be reconstructed exactly. This may not be a problem in some applications. For example, a small quality loss when reconstructing audio files may be tolerated by a human listener. 

### Image compression 

Image compression refers to data compression applied to digital images. 
Images carry a huge amount of information and can occupy a significant amount of disk space. 

On the internet, they also take up to 65% of most webpages [2](Compression Techniques, n.d.). Therefore, good image compression algorithms can speed up page rendering, reduce bandwitdh and save battery life for mobile devices. 

There are both lossless and lossy methods of compressing images. Lossy image compression might be unsuitable for a number of tasks like medical imaging [3](Wong et al., 1995), satelite imagery and scientific illustrations and diagrams.
This paper will focus on a lossless compression scheme.

### What makes images compressible

[TODO]

### File formats

A file format defines how data inside a file is  arranged. This includes things like how text characters are encoded, or how audio is stored as digital waveforms. An image format is a file format for a digital image.

The image format dictates how pixels are stored in a file. Image data can be stored compressed or uncompressed. Examples of popular image formats are: PNG, JFIF, WEBP, QOI, BMP and TIFF.

Image formats may use lossless or lossy compression. 
For example, the PNG standard specifies that the compression should preserve all information [4](Portable Network Graphics (PNG) Specification (Second Edition), n.d.). JFIF, on the other hand, uses JPEG compression, which can be either lossless or lossy.

### Codes 

In information theory, a code refers to a system of mapping symbols or strings of symbols to codewords, where each codeword is a string of bits. For text data, symbols may be individual characters like letters, numbers and punctuation. For image data, symbols could represent individual pixels in an image.

Using codes, we can map any information into a bitstring. The length of the bitstring depends on the quality of the code and the probabilities of the individual symbols [5].

### FELICS 

FELICS, which stands for "fast and efficient lossless image compression system", works by modeling the distribution of a pixel's intensity value using the values of its two nearest neighbours that have already been visited. 

FELICS proceeds by coding pixels their in raster-scan order. This means that FELICS traverses the image line by line, from the left to the right. Therefore, the two nearest neighbours of a pixel are usually the one above and the one to the left of the current pixel. 

![neighbour-figure](./figures/neighbours.png)

*Figure shows the various possible configurations for the neighbouring pixels (A and B) of a given pixel (X)*

In the context of a grayscale image, each pixel has a single intensity value, $ V $. For images with multiple channels, each pixel may be represented by multiple intensity values. For example, an RGB image has three channels: red, green and blue. We can think of a pixel as a triplet $ (R, G ,B) $, with an intensity value for each channel.
Since the algorithm only works for grayscale images, a pixel will only have one intensity value. 

To encode a pixel $P$, the algorithm looks at the two neighbouring pixels and their intensities. The smaller neighbouring value is called $ L $, and the larger value $ H $. Next, we compute $ \Delta = H - L$, the prediction context of $P$. The coding proceeds as follows:

<pre>
if  L <= P <= H    
    use one bit to encode IN-RANGE 
    encode the value P - L in the range [0, Δ] using a truncated binary code 
if P < L
    use one bit to encode OUT-OF-RANGE
    use one bit to encode BELOW-RANGE 
    encode the value L-P-1 using Golomb-Rice codes 
if P > H
    use one bit to encode OUT-OF-RANGE
    use one bit to encode ABOVE-RANGE 
    encode the value P-H-1 using Golomb-Rice codes 
</pre>

The first two pixels in the image are outputed without coding. The steps above are then repeated for every pixel in the image, in raster-scan order.

## Bibliography
1) Sayood, K. (2006). Introduction to data compression (3rd ed.). Elsevier.

2) Compression techniques. (n.d.). Google for Developers. https://developers.google.com/speed/webp/docs/compression

3) Wong, S. L., Zaremba, L., Gooden, D. S., & Huang, H. K. (1995). Radiologic image compression-a review. Proceedings of the IEEE, 83(2), 194–219. https://doi.org/10.1109/5.364466

4) Portable Network Graphics (PNG) Specification (Second Edition). (n.d.). https://www.w3.org/TR/2003/REC-PNG-20031110/

5) Salomon, D. (2007). Variable-length codes for data compression. In Springer eBooks. https://doi.org/10.1007/978-1-84628-959-0

6) Howard, P. G., & Vitter, J. S. (n.d.). Fast and efficient lossless image compression. In [Proceedings] DCC '93: Data Compression Conference. [Proceedings] DCC '93: Data Compression Conference. IEEE Comput. Soc. Press. https://doi.org/10.1109/dcc.1993.253114