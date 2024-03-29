
import time
import sys
import math

from rgbmatrix import RGBMatrix, RGBMatrixOptions, graphics
from PIL import Image

def get_matrix():
    # Configuration for the matrix
    options = RGBMatrixOptions()
    options.rows = 16
    options.cols = 32
    options.chain_length = 1
    options.parallel = 1
    # Note: The guide in
    # https://learn.adafruit.com/neopixels-on-raspberry-pi/raspberry-pi-wiring
    # uses the PWM and DMA features to drive _neopixels_ - which means we can't
    # use PWM to improve the quality of this display. Drat!
    options.hardware_mapping = 'adafruit-hat'
    # options.show_refresh_rate = 1

    return RGBMatrix(options = options)

def color_of(brmin, brmax, hour, minute, second):
    """Produces a unique color for a given time.

    The minute and second produce a hue value; the hue cyles on an hourly basis.

    The hour modulates the brightness, between brmin (at midnight) and brmax (at noon). brmin and brmax are in the range [0, 1].

    Converts a hue value in degrees (minute, second) to an RGB tuple.
    """
    angle = minute * 6 + (second / 10)

    def _f(n, h, s, l):
      k = (n + h / 30) % 12
      a = s * min(l, 1-l)
      minterm = min(k - 3, 9 - k)
      return l - a * max(-1, min(minterm, 1))

    def _rgb(h, s, l):
      r = _f(0, h, s, l) * 255
      g = _f(8, h, s, l) * 255
      b = _f(4, h, s, l) * 255
      return graphics.Color(r, g, b)

    # Daily cycle: scale hour to radians.
    brangle = (hour / 24.0) * 2.0 * math.pi
    # Minimum at 0, maximum at noon- that's -cos.
    # Scale into (0, 1).
    brscale = (1 - math.cos(brangle)) / 2
    # Scale further, from brmin to brmax.
    # Scale down so we're dealing with "luminant" colors, not washed-out-
    # that is, limit to the value regime.
    luminance = (brmin + brscale * (brmax - brmin)) * 0.5
    return _rgb(angle, 1.0, luminance)

class Walker:
    def __init__(self, matrix: RGBMatrix):
        self._matrix = matrix
        self._canvas = self._matrix.CreateFrameCanvas()
        self._font = graphics.Font()
        self._font.LoadFont("5x13.bdf")
        # The observed dimensions of the numbers of this font:
        self._font_width = 5
        self._font_height = 9

    def _render(self):
        canvas = self._canvas

        canvas.Clear()

        now = time.localtime()
        hour = now.tm_hour
        minute = now.tm_min

        color = color_of(0.1, 0.5, hour, minute, now.tm_sec)

        text = "{:02d}:{:02d}".format(hour, minute)
        # Start at Y offset equal to font height, otherwise it draws offscreen.
        #
        # The fonts provided with rgbmatrix have an odd baseline- always 1px,
        # with descenders hitting the same height as the bottom of other
        # characters.
        # Trying out options from https://leahneukirchen.org/fonts/ instead.
        textwidth = len(text) * self._font_width
        # Empty column is on the RHS of the character, so we round the LH margin up
        h_margin = math.ceil((canvas.width - textwidth) / 2)

        graphics.DrawText(canvas, self._font, h_margin, self._font_height + 1, color, text)

        self._canvas = self._matrix.SwapOnVSync(canvas)

    def run(self):
        while True:
            self._render()
            time.sleep(0.1)

if __name__ == "__main__":
    walker = Walker(get_matrix())
    walker.run()

