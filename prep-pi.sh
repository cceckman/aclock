#!/bin/sh

set -e

TARGET="$1"
if ! test "$#" -eq 1
then
  echo >&2 wrong number of arguments: need path of boot partition
  exit 1
fi

if ! test -f "$TARGET"/config.txt
then
  echo >&2 "$TARGET" does not appear to be the firmware partition of an RPI image
  exit 1
fi

sudo -n sh -c "echo >&2 OK to use sudo" || {
  echo >&2 "enable passwordless sudo before continuing"
  exit 1
}

echo >&2 "editing $TARGET"

sudo -n sed -i "s/^.*dtparam=spi=on/dtparam=spi=on/" "$TARGET/config.txt"
if ! grep -q 'dtparam=spi=on' "$TARGET/config.txt"
then
 echo 'dtparam=spi=on' | sudo tee -a "$TARGET/config.txt"
fi

cat wpa_supplicant.conf | sudo tee "$TARGET"/wpa_supplicant.conf >/dev/null
sudo touch "$TARGET"/ssh

sudo umount "$TARGET"
