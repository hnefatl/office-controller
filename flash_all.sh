#!/bin/bash
#
# `espflash` doesn't support nicely flashing additional NVS partitions: you need to parse the partition table
# again and pass the offset in. It also doesn't support building a data NVS partition.
#
# Since some of these tasks need the esp-idf repository to do properly, just handle all the building and
# flashing consistently using esp-idf.

set -o errexit -o pipefail

source esp_idf_common.sh
source "${ESP_IDF_DIR}/export.sh"

./flash_config.sh

export IDF_EXPORT_QUIET=true
cd "${ESP_IDF_DIR}"

echo "Converting application"
esptool.py --chip=esp32 elf2image ../target/xtensa-esp32-espidf/debug/office-controller --output ../target/office_controller.bin
echo "Writing application"
parttool.py --esptool-args="chip=esp32" --partition-table-file=../partitions.csv write_partition --partition-name=factory --input="../target/office_controller.bin"

cargo espflash monitor --after=hard-reset
