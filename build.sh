#!/bin/sh
TARGET=arm-unknown-linux-musleabi

if [ ! -d "./musl" ]; then
	if [ ! -f "./arm-linux-musleabi-cross.tgz" ]; then
		wget http://lisa.musl.cc/9.3.0/arm-linux-musleabi-cross.tgz
	fi
	mkdir ./musl
	tar -C ./musl -x -f arm-linux-musleabi-cross.tgz
	sudo xattr -r -d com.apple.quarantine ./musl
	#rm arm-linux-musleabi-cross.tgz
fi

PATH=$PATH:$(pwd)/musl/arm-linux-musleabi-cross/bin cargo build --release --target $TARGET