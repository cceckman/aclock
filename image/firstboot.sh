#!/bin/sh

set -e
# First-boot setup that Raspbian doesn't like to let us do itself.

while ! test -d /home/cceckman/
do
  sleep 30
done

su cceckman -c "if ! test -d ~/.ssh; then mkdir -m 0700 ~/.ssh; fi"
su cceckman -c "touch ~/.ssh/authorized_keys"
su cceckman -c "chmod 0755 ~/.ssh/authorized_keys"
su cceckman -c "echo 'sk-ssh-ed25519@openssh.com AAAAGnNrLXNzaC1lZDI1NTE5QG9wZW5zc2guY29tAAAAIFhsrsc3V3KcH79keRp/jL38ty/BHh5897avu2hMvthJAAAABHNzaDo= cceckman@cromwell' >~/.ssh/authorized_keys"

# Recommended in
# https://github.com/hzeller/rpi-rgb-led-matrix/tree/master?tab=readme-ov-file#use-minimal-raspbian-distribution
apt-get remove -y bluez bluez-firmware pi-bluetooth triggerhappy pigpio

apt-get install -y vim

systemctl disable cce-firstboot
