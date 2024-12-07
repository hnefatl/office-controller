#!/bin/bash
#
# `espflash` doesn't support nicely flashing additional NVS partitions: you need to parse the partition table
# again and pass the offset in. It also doesn't support building a data NVS partition.
#
# Since some of these tasks need the esp-idf repository to do properly, just handle all the building and
# flashing consistently using esp-idf.

set -o errexit -o pipefail

./flash_config.sh
exec espflash flash --chip=esp32 --partition-table=partitions.csv --target-app-partition=factory --monitor "$@"
