#!/bin/bash

# based on https://fast-dds.docs.eprosima.com/en/latest/installation/sources/sources_linux.html
set -e

defaultinst=~/Fast-DDS/install/
echo Installing to ${INSTALL_DIR:=$defaultinst}

cd ~/Fast-DDS

cd foonathan_memory_vendor/build
cmake .. -DCMAKE_INSTALL_PREFIX=${INSTALL_DIR} -DBUILD_SHARED_LIBS=ON
cmake --build . --target install

cd ~/Fast-DDS
cd Fast-CDR/build
cmake .. -DCMAKE_INSTALL_PREFIX=${INSTALL_DIR} -DBUILD_SHARED_LIBS=ON
cmake --build . --target install
