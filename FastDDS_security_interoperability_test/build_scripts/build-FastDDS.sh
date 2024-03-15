#!/bin/bash

# based on https://fast-dds.docs.eprosima.com/en/latest/installation/sources/sources_linux.html
set -e

defaultinst=~/Fast-DDS/install/
echo Installing to ${INSTALL_DIR:=$defaultinst}

buildroot=~/Fast-DDS
cd $buildroot
echo buildroot is ${buildroot}

if [[ $1 = "m" ]]; then
    cd Fast-DDS-with-security-interoperability-modifications/build
elif [[ $1 = "u" ]]; then
    cd Fast-DDS/build
else 
    echo "Provide argument m or u to choose the modified or unmodified FastDDS repository."
    exit 1
fi
cmake   -DSECURITY:BOOL=ON \
        -DSHM_TRANSPORT_DEFAULT:BOOL=OFF \
        -DLOG_NO_INFO:BOOL=OFF \
        -DFASTDDS_ENFORCE_LOG_INFO:BOOL=ON \
        -DBUILD_SHARED_LIBS=ON \
        -DCMAKE_INSTALL_PREFIX=${INSTALL_DIR} \
        --debug-find \
        ..

cmake --build . --target install

#         -DCMAKE_PREFIX_PATH="${buildroot}/Fast-CDR;${buildroot}/foonathan_memory_vendor;${buildroot}/install" \

#        -Dfastcdr_DIR=${buildroot}/Fast-CDR \
#        -Dfoonathan_memory_DIR=${INSTALL_DIR}/lib/foonathan_memory \
