#!/bin/bash

set -o errexit -o pipefail

source esp_idf_common.sh

export IDF_EXPORT_QUIET=true
source "${ESP_IDF_DIR}/export.sh"

python "${ESP_IDF_DIR}/components/nvs_flash/nvs_partition_generator/nvs_partition_gen.py" \
    generate config_partition.csv target/config_partition.bin 0x3000

export IDF_EXPORT_QUIET=true
cd "${ESP_IDF_DIR}"

echo "Writing config"
parttool.py --esptool-args="chip=esp32" --partition-table-file=../partitions.csv write_partition --partition-name=config  --input=../target/config_partition.bin
