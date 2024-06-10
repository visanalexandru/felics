from PIL import Image
import numpy as np

im = np.array(Image.open("peppers.tiff"))
print(im[:, :, 2])
