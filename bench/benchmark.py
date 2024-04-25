import subprocess
import os
from time import time
import matplotlib.pyplot as plt

# Returns the size of the directory on disk, in mb.
def get_disk_usage(path):
    output = subprocess.check_output(['du','-m', '-s', path])
    size, _ = output.split()
    size = size.decode()
    return int(size)

def to_png(input_file, output_file):
    return ["convert", f"tiff_files/{input_file}",  "-quality",  "100",  f"to_png/{output_file}"]

def to_webp(input_file, output_file):
    return ["cwebp", "-lossless",  f"tiff_files/{input_file}",  "-o",  f"to_webp/{output_file}"]

def to_felics(input_file, output_file):
    return ["cfelics",  "-i",  f"tiff_files/{input_file}",  "-o",  f"to_felics/{output_file}"]

def to_qoi(input_file, output_file):
    return ["convert", f"tiff_files/{input_file}", f"to_qoi/{output_file}"]

def from_png(input_file, output_file):
    return ["convert", f"to_png/{input_file}",  f"from_png/{output_file}"]

def from_webp(input_file, output_file):
    return ["dwebp", f"to_webp/{input_file}",  "-o", f"from_webp/{output_file}", "-tiff"]

def from_felics(input_file, output_file):
    return ["dfelics", "-i", f"to_felics/{input_file}",  "-o", f"from_felics/{output_file}"]

def from_qoi(input_file, output_file):
    return ["convert", f"to_qoi/{input_file}", f"from_qoi/{output_file}"]


# Compresses the given list of files, using the given command.
def benchmark_compression(input_files, format, output_dir, compression_command):
    print(f"Benchmarking compression for: {format}")
    start = time()
    for input_file in input_files:
        filename, _ = os.path.splitext(input_file)
        output_file = filename + format

        to_call = compression_command(input_file, output_file)
        print(f"{input_file} -> {output_file}, command: {to_call}")
        subprocess.run(to_call)
    end = time()

    size = get_disk_usage(output_dir)
    print()
    return end-start, size

def benchmark_decompression(input_files, format, decompresion_command):
    print(f"Benchmarking decompression for: {format}")
    start = time()

    for input_file in input_files:
        filename, _ = os.path.splitext(input_file)
        output_file = filename + ".tiff" 

        to_call = decompresion_command(input_file, output_file)
        print(f"{input_file} -> {output_file}, command: {to_call}")
        subprocess.run(to_call)

    end = time()
    print()
    return end-start

# Create output directories if they do not exist.
def create_output_dirs(dirs):
    for dir in dirs:
        if not os.path.exists(dir):
            os.mkdir(dir)

def plot_compression_metrics():
    files_to_compress = [x for x in os.listdir("tiff_files") if x.endswith(".tiff")]
    output_dirs = ["to_png/", "to_webp/", "to_felics/", "to_qoi/"]
    create_output_dirs(output_dirs)

    formats = [".png", ".webp", ".fel", ".qoi"]
    commands = [to_png, to_webp, to_felics, to_qoi]

    times = []
    usages = []

    for to_format, output_dir, command in zip(formats, output_dirs, commands):
        time_taken, memory_used = benchmark_compression(files_to_compress, to_format, output_dir, command)
        times.append(time_taken)
        usages.append(memory_used)

    print("Compression times: ", list(zip(formats, times)))
    print("Memory usages: ", list(zip(formats, usages)))

    plt.ylabel("Compression elapsed time (seconds)")
    plt.bar(formats, times)
    plt.show()

    plt.ylabel("Size (MB)")
    plt.bar(formats, usages)
    plt.show()

def plot_decompression_metrics():
    input_formats = [".png", ".webp", ".fel", ".qoi"]
    input_dirs = ["to_png/", "to_webp/", "to_felics/", "to_qoi/"]
    output_dirs = ["from_png/", "from_webp/", "from_felics/", "from_qoi/"]
    commands = [from_png, from_webp, from_felics, from_qoi]

    create_output_dirs(output_dirs)

    times = []

    for input_format, input_dir, command in zip(input_formats, input_dirs, commands):
        files_to_decompress = [x for x in os.listdir(input_dir) if x.endswith(input_format)]
        time_taken = benchmark_decompression(files_to_decompress, input_format, command)
        times.append(time_taken)

    print("Decompression times: ", list(zip(input_formats, times)))

    plt.ylabel("Compression elapsed time (seconds)")
    plt.bar(input_formats, times)
    plt.show()


if __name__ == "__main__":
    plot_compression_metrics()
    plot_decompression_metrics()

