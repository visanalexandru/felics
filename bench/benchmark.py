import subprocess
import os
from time import time
import matplotlib.pyplot as plt

# Returns the size of the folder on disk, in mb.
def get_disk_usage(path):
    output = subprocess.check_output(['du','-m', '-s', path])
    size, _ = output.split()
    size = size.decode()
    return int(size)

def to_png(input_file, output_file):
    return ["convert", input_file,  "-quality",  "100",  "to_png/"+ output_file]

def to_webp(input_file, output_file):
    return ["cwebp", "-lossless",  input_file,  "-o",  "to_webp/"+output_file]

def to_felics(input_file, output_file):
    return ["cfelics",  "-i",  input_file,  "-o",  "to_felics/"+output_file]

def to_qoi(input_file, output_file):
    return ["convert", input_file, "to_qoi/"+ output_file]

if __name__ == "__main__":
    # Create output directories if they do not exist.
    output_dirs = ["to_png/", "to_webp/", "to_felics/", "to_qoi/"]
    for output_dir in output_dirs:
        if not os.path.exists(output_dir):
            os.mkdir(output_dir)

    files_to_convert = [x for x in os.listdir("tiff_files") if x.endswith(".tiff")]

    to_formats = [".png", ".webp", ".fel", ".qoi"]
    commands = [to_png, to_webp, to_felics, to_qoi]

    elapsed = []

    for (to_format, command) in zip(to_formats, commands):
        print(f"Converting all files to: {to_format}")

        start = time()

        for input_file in files_to_convert:
            filename, ext = os.path.splitext(input_file)
            output_file = filename + to_format 

            to_call = command("tiff_files/"+input_file, output_file)
            print(f"{input_file} -> {output_file}, command: {to_call}")
            subprocess.run(to_call)

        end = time()

        print(f"Took: {end-start}s\n")
        elapsed.append(end-start)

    plt.ylabel("Elapsed time (seconds)")
    plt.bar(to_formats, elapsed)
    plt.show()

    plt.ylabel("Size (MB)")
    plt.bar(to_formats, [get_disk_usage("to_png/"), get_disk_usage("to_webp/"), get_disk_usage("to_felics/"), get_disk_usage("to_qoi/")])
    plt.show()
