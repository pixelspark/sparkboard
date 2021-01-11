# Sparkboard

## Building

Regular build:
````sh
cargo build
````

For Raspberry Pi:
````sh
cargo build --target=arm-unknown-linux-musleabi
````

## Usage

````sh
# Run a test
RUST_LOG=debug ./sparkboard -t 7in5v2 -d /dev/spidev0.0 test

# Show a PNG image (must be 800x480 for the v2 display)
RUST_LOG=info ./sparkboard -t 7in5v2 fetch "http://example.com/image-800x480.png"

# Show a PNG image (must be 640x384 for the bc display)
RUST_LOG=info ./sparkboard -t 7in5bc fetch "http://example.com/image-640x384.png"
````
