#!/bin/bash
#!/bin/bash

cargo build --release
sudo cp target/release/undervolt-gui /usr/local/bin/
sudo cp *.desktop /usr/share/applications/
