#!/bin/sh

PROJECT=$(pwd)
SYSROOT_SSL=$PROJECT/../compile/libraries/openssl/
SYSROOT_ALSA=$PROJECT/../compile/libraries/alsa/
SYSROOT_OPUS=$PROJECT/../compile/libraries/opus/

export PKG_CONFIG_DIR=
export PKG_CONFIG_PATH=${SYSROOT_SSL}lib/pkgconfig:${SYSROOT_ALSA}lib/pkgconfig:${SYSROOT_OPUS}lib/pkgconfig:
export PKG_CONFIG_LIBDIR=${SYSROOT_SSL}lib/pkgconfig:${SYSROOT_ALSA}lib/pkgconfig/:${SYSROOT_OPUS}lib/pkgconfig/:
export PKG_CONFIG_SYSROOT_DIR=${SYSROOT_SSL}:${SYSROOT_ALSA}:${SYSROOT_OPUS}:
export PKG_CONFIG_ALLOW_CROSS=1
export OPENSSL_DIR=${SYSROOT_SSL}
#pkg-config --variable pc_path pkg-config

cd ${PROJECT} && RUSTFLAGS='-L '$SYSROOT_SSL'/lib/ -L '$SYSROOT_ALSA'/lib/ -L '$SYSROOT_OPUS'/lib/' cargo build --release --target $TARGET --verbose 
