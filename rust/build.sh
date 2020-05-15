#!/bin/sh
tmux renamew -t $TMX_WINID building...
clear
if exectime cargo build --release; then
cp target/release/libboids.so ../godot/lib/libboids.so
fi

