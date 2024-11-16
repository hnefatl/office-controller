#!/bin/bash
#
# Initialise the esp-idf tools part of the repository.

set -o errexit -o pipefail

cd "$(git rev-parse --show-toplevel)"

source esp_idf_common.sh
ESP_IDF_VERSION="v5.1.5"

if [ ! -d "${ESP_IDF_DIR}" ] ; then
    echo "${ESP_IDF_DIR} does not exist, cloning..."
    git clone --quiet --depth 1 --single-branch --branch "${ESP_IDF_VERSION}" https://github.com/espressif/esp-idf "${ESP_IDF_DIR}"
else
    echo "${ESP_IDF_DIR} already exists, ensuring up to date..."
    git -C "${ESP_IDF_DIR}" pull --quiet origin "${ESP_IDF_VERSION}"
fi

echo "Installing/updating python deps in virtual env..."

cd "${ESP_IDF_DIR}"
./install.sh --targets=esp32
