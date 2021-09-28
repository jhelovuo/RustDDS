#!/bin/bash
echo Pausing outbound multicast
iptables --out-interface ens192 --insert OUTPUT --destination 239.255.0.1  \
 --protocol udp --destination-port 7400 -j DROP

echo sleeping...
sleep 6

echo Resuming multicast
iptables --delete OUTPUT 1
