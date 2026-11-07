#!/bin/bash

cargo build --release
sudo cp -f target/release/undervolt-gui /usr/local/bin/
sudo cp -f *.desktop /usr/share/applications/
cp -f *.desktop ~/.local/share/applications/
