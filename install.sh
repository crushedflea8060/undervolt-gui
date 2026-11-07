#!/bin/bash
#!/bin/bash

cargo build --release
sudo mv target/release/undervolt-gui /usr/local/bin/
sudo mv *.desktop /usr/share/applications/
