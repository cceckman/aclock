# aclock

Files to make an overcomplicated clock.

I want a clock that indicates:

- The time
- Daylight / nighttime hours
- General telemetry status for the rest of the
  [homelab](https://github.com/cceckman/homelab)

The general layout is to have a big (digital) display in the center, and use
LEDs around the edge to indicate the daylight/nighttime hours.

![Screen over the source of a year](year.webp)

## Active hardware

- Raspberry Pi of some flavor
- 1 meter of RGBW LED strip with individually-addressible LEDs (NeoPixel).

  [Product page](http://www.adafruit.com/product/2846),
  [Datasheet](docs/SK6812RGBW.pdf)

- 16x32 RGB LED matrix, 6mm pitch (so, 192mm x 6mm).

  [Product page](https://www.adafruit.com/product/420),
  [Datasheet](docs/P420_Indoor-P6-8S-16x32-SMD3528.pdf)

- LED matrix + RTC HAT - driver board for the matrix, sized to Raspberry Pi

  [Product page](http://www.adafruit.com/product/2345),
  [Matrix driver library](https://github.com/hzeller/rpi-rgb-led-matrix)

- I2c CO2 / Temp / Humidity sensor

  I have a Rust driver for this kicking around somewhere, but there's also the
  Adafruit library.

  [Product page](https://www.adafruit.com/product/4867),
  [Datasheet](docs/SCD30.pdf),
  [Library source](https://github.com/adafruit/Adafruit_CircuitPython_SCD30),
  [Model](https://github.com/adafruit/Adafruit_CAD_Parts/tree/main/4867%20SCD-30%20C02%20Sensor)

## Passive hardware

I'm treating this as an opportunity to learn a bit of CAD, and spin up a 3d
printer.

- Central, rectangular frame: holds LED matrix in the center,
  Pi / HAT / SCD30 below

  Based on some other prints, this may be too big / take too long to be
  practical; I may want to split it up. And see dimension considerations below.

- Clip-ons to the center frame to make the edge round, and mount the LED strip
  on it

Playing around with some dimensions - ignoring print width:

- 1m radius gives 318.3mm diameter. We can have a little give /gap at the bottom
  and make it a round 320mm diameter.
- 320mm becomes the diagonal of an inscribed square;
  `320mm / sqrt(2) = 226.27mm` as the side of the square.
- We take 96mm out of that for the LED matrix; and it's centered, so we only get
  half of the remainder to fit in the RPi. That's about 65mm; a Pi board is
  56.5mm wide (86mm-odd long). That gives us not quite 10mm of framing space to
  work with.

## Software

Given that the libraries seem to be available in Python, it may be easiest to
start there.

[Matrix driver library](https://github.com/hzeller/rpi-rgb-led-matrix)
for running the clock face. It's actually C++, with Python bindings- so will
require "manual" install. There's a [reference install
script](https://raw.githubusercontent.com/adafruit/Raspberry-Pi-Installer-Scripts/main/rgb-matrix.sh) but- it's interactive, so, no.

[Astral](https://astral.readthedocs.io/en/latest/) for computing rise/set times

[Adafruit-Blinka](https://pypi.org/project/Adafruit-Blinka/)
provides a
compatibility layer, adding CircuitPython to regular Python (per
[their guide](https://learn.adafruit.com/circuitpython-on-raspberrypi-linux))
for the Neopixel library.

The [Neopixel
guide](https://learn.adafruit.com/neopixels-on-raspberry-pi/python-usage) points
to `rpi_ws281x` and `adafruit-circuitpython-neopixel` from PIP as the other
sources.

