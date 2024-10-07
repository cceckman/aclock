
set -eu
redo-ifchange rpi-64.img

cp rpi-64.img "$3"

sudo -n "echo" >/dev/null || {
    echo >&2 "Need noninteractive sudo to complete, exiting"
    exit 1
}

if test -e ./mount
then
    sudo -n umount -Rd mount/ || true
    rm -rf ./mount
fi

mkdir -p mount/

# Set up mountpoint:
LODEV="$(sudo -n losetup --show -P -f "$3")"
sudo -n mount -o loop "$LODEV"p2 mount/
sudo -n mount -o loop "$LODEV"p1 mount/boot/firmware/

redo-ifchange wpa_supplicant.conf userconf.txt firstboot.sh cce-firstboot.service config.txt

sudo -n cp userconf.txt mount/boot/firmware/userconf.txt
echo >&2 "Updating config.txt:"
diff >&2 mount/boot/firmware/config.txt config.txt || true
sudo -n cp config.txt mount/boot/firmware/config.txt

# For better LED matrix performance:
# https://access.redhat.com/solutions/480473
echo -n " isolcpus=3" >>mount/boot/firmware/cmdline.txt
# For SPI use for NeoPixel driver:
# https://github.com/jgarff/rpi_ws281x?tab=readme-ov-file#spi
echo -n " spidev.bufsiz=32768" >>mount/boot/firmware/cmdline.txt


sudo -n cp wpa_supplicant.conf mount/etc/wpa_supplicant/wpa_supplicant.conf
sudo -n mkdir -p mount/opt/
sudo -n cp firstboot.sh mount/opt/firstboot.sh
sudo -n cp cce-firstboot.service mount/etc/systemd/system/cce-firstboot.service
sudo -n ln -s /etc/systemd/system/cce-firstboot.service mount/etc/systemd/system/multi-user.target.wants/cce-firstboot.service

# Remove mountpoint (-R ecursive -d etach loop device)
sudo -n umount -Rd mount/
sync

echo >&2 "Baseline image ready"