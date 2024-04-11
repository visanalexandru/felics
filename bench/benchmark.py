import subprocess
import os
from time import time

output_dirs = ["to_png/", "to_webp/", "to_felics/", "to_qoi/"]
for output_dir in output_dirs:
    if not os.path.exists(output_dir):
        os.mkdir(output_dir)

def to_png(input_file, output_file):
    return ["convert", input_file,  "-quality",  "100",  "to_png/"+ output_file]

def to_webp(input_file, output_file):
    return ["cwebp", "-lossless",  input_file,  "-o",  "to_webp/"+output_file]

def to_felics(input_file, output_file):
    return ["cfelics",  "-i",  input_file,  "-o",  "to_felics/"+output_file]

def to_qoi(input_file, output_file):
    return ["convert", input_file, "to_qoi/"+ output_file]

to_formats = [".png", ".webp", ".fel", ".qoi"]
commands = [to_png, to_webp, to_felics, to_qoi]

files_to_convert = [x for x in os.listdir() if x.endswith(".jpg")]

for (to_format, command) in zip(to_formats, commands):
    print(f"Converting all files to: {to_format}")

    start = time()

    for input_file in files_to_convert:
        filename, ext = os.path.splitext(input_file)
        output_file = filename + to_format 

        to_call = command(input_file, output_file)
        print(f"{input_file} -> {output_file}, command: {to_call}")
        subprocess.run(to_call)

    end = time()

    print(f"Took: {end-start}s\n")



