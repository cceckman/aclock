#! /bin/sh
#
# Deploy an update just to the source files.

ansible-playbook -i ../homelab/inventory.yaml install.yaml \
  -e quick=true
