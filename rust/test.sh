#!/bin/sh
if ! [ -x "$(command -v godot-headless)" ]; then
    echo "No godot-headless."
    echo "Download headless from https://godotengine.org/download/server"
    echo "and rename it to godot-headless and place it in your PATH"
    exit 1
fi

if cargo build --release; then
    cp target/release/libboids.so ../test/lib/libboids.so
    cd ../test && godot-headless; then
fi
