#!/bin/bash

set -o errexit -o pipefail

if [[ ! -d .venv ]] ; then
    python3 -m venv .venv
    source .venv/bin/activate
    pip3 install esptool
fi

source .venv/bin/activate
# The rust espflash tool's `write-bin` command doesn't actually write any data.
# Potentially https://github.com/esp-rs/espflash/issues/622.
# I cba trying to fix the tool because it's structured terribly.
esptool.py --chip=esp32 write_flash $((16#9000)) deployment_config.postcard
