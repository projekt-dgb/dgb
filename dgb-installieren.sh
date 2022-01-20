#!/bin/bash

if [[ $EUID -ne 0 ]]; then
    echo "$0 ist nicht root. Bitte \"sudo / doas ./dgb-installieren.sh\" verwenden."
    exit 2
fi

chmod +x ./dgb_1.0.0_amd64.deb
dpkg -i ./dgb_1.0.0_amd64.deb
