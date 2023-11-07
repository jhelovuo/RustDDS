#!/bin/bash

# based on https://fast-dds.docs.eprosima.com/en/latest/installation/sources/sources_linux.html

if [ $EUID -ne 0 ]; then
    echo "Please run using sudo."
    exit 2
fi

cd /home/$SUDO_USER/Fast-DDS

if [ $1="m" ]; then
    cd Fast-DDS-with-security-interoperability-modifications/build
elif [ $1="u" ]; then
    cd Fast-DDS/build
else 
    echo "Provide argument m or u to choose the modified or unmodified FastDDS repository."
    exit 1
fi
cmake -DSECURITY:BOOL=ON ..  -DCMAKE_INSTALL_PREFIX=/usr/local/ -DBUILD_SHARED_LIBS=ON
cmake --build . --target install

