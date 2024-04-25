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

# Compresses the given list of files, using the given command.
def benchmark_compression(input_files, format, output_dir, compression_command):
    print(f"Benchmarking: {format}")
    start = time()
    for input_file in input_files:
        filename, _ = os.path.splitext(input_file)
        output_file = filename + format

        to_call = compression_command(input_file, output_file)
        print(f"{input_file} -> {output_file}, command: {to_call}")
        subprocess.run(to_call)
    end = time()

    size = get_disk_usage(output_dir)
    
    return end-start, size

# Create output directories if they do not exist.
def create_output_dirs(dirs):
    for dir in dirs:
        if not os.path.exists(dir):
            os.mkdir(dir)

if __name__ == "__main__":
    files_to_convert = [x for x in os.listdir("tiff_files") if x.endswith(".tiff")]
    print(f"Input files: {files_to_convert}")

    output_dirs = ["to_png/", "to_webp/", "to_felics/", "to_qoi/"]
    create_output_dirs(output_dirs)

    formats = [".png", ".webp", ".fel", ".qoi"]
    commands = [to_png, to_webp, to_felics, to_qoi]

    times = []
    usages = []

    for to_format, output_dir, command in zip(formats, output_dirs, commands):
        time_taken, memory_used = benchmark_compression(files_to_convert, to_format, output_dir, command)
        times.append(time_taken)
        usages.append(memory_used)

    plt.ylabel("Compression elapsed time (seconds)")
    plt.bar(formats, times)
    plt.show()

    plt.ylabel("Size (MB)")
    plt.bar(formats, usages)
    plt.show()

