
import time
import sys

from rgbmatrix import RGBMatrix, RGBMatrixOptions
from PIL import Image

def get_matrix():
    # Configuration for the matrix
    options = RGBMatrixOptions()
    options.rows = 16
    options.cols = 32
    options.chain_length = 1
    options.parallel = 1
    # TODO: strap PWM pins for smooth running
    options.hardware_mapping = 'adafruit-hat'
    options.show_refresh_rate = 1

    return RGBMatrix(options = options)

class Walker:
    def __init__(self, matrix: RGBMatrix):
        self._matrix = matrix
        self._x = 0
        self._y = 0

    def _step(self, canvas):
        canvas.Fill(0, 0, 0)

        # Move to the next pixel:
        self._x += 1
        if self._x == canvas.width:
            self._x = 0
            self._y = (self._y + 1) % canvas.height
        canvas.SetPixel(self._x, self._y, 128, 128, 128)

        # Always render the zero coordinate, for orientation
        canvas.SetPixel(0, 0, 128, 0, 0)

        return self._matrix.SwapOnVSync(canvas)

    def run(self):
        canvas = self._matrix.CreateFrameCanvas()
        while True:
            canvas = self._step(canvas)

if __name__ == "__main__":
    walker = Walker(get_matrix())
    walker.run()

