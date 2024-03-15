#!/bin/bash

# based on https://fast-dds.docs.eprosima.com/en/latest/installation/sources/sources_linux.html

if [[ $1 != "u" && $1 != "m" ]]; then
    echo "Please give argument m or u"
    exit 1
fi

./clone-repositories.sh $1
./build-dependencies.sh
./build-FastDDS.sh $1
