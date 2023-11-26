# TODO list

## 1. Bit manipulation 

The FELICS compression algorithm relies heavily on the ability to write bit-level data: 

- Encoding the range of a pixel P:
    - It takes 1 bit to encode if P lies in the range $ \left[L, H\right] $
    - If P lies outside the range, it takes another bit for the above-range/below-range decision

- Encoding an in-range pixel value:

    In-range values are almost uniform. Let $ \Delta $ be $ H - L $.

    If $ \Delta + 1 $ is a power of two we simply use a binary code with $ \log_2(\Delta + 1) $	bits.

    Otherwise, we can assign $\lfloor \log_2(\Delta + 1) \rfloor$	to some values and $\lceil \log_2(\Delta + 1) \rceil$ to others. This wikipedia article describes how to achieve this: [truncated binary encoding](https://en.wikipedia.org/wiki/Truncated_binary_encoding). 

- Encoding an out-of-range pixel value using Rice coding. This operation depends on the rice coding parameter.

Also, we must be able to decode bit-level data. This is necessary for decompression:

- Reading the next bit to find the range of a pixel P.

- Decoding an in-range pixel value.


### Implementation:

1. Create a BitWriter data structure that supports these operations:
    - ``` Push(bit) ``` - pushes a new bit into the bit-sink.
    - ``` PushN(bits, n) ``` - pushes multiple bits into the bit-sink.
    - ``` Flush() ``` - flushes the buffered data into the underlying container.
    - The underlying container will be a struct that implements ```std::io::Write```

2. Create a RangeWriter data structure that supports these operations:
    - ``` EncodeRange(range) ``` - pushes one bit if IN_RANGE and two if ABOVE_RANGE or BELOW_RANGE to the underlying ```BitWriter```.

3. Create a PixelWriter data structure that supports these operations:
    - ```EncodePixel(value) ``` - pushes the encoded representation of the given value to the underlying ```BitWriter```.

        This will depend on the pixel being IN_RANGE, ABOVE_RANGE or BELOW_RANGE.
