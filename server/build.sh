#!/bin/sh

SCRIPT=`realpath $0`
SCRIPTPATH=`dirname $SCRIPT`

SYSROOT=$SCRIPTPATH/libopus/

export PKG_CONFIG_DIR=
export PKG_CONFIG_LIBDIR=$PKG_CONFIG_LIBDIR:${SYSROOT}lib/pkgconfig
export PKG_CONFIG_SYSROOT_DIR=$PKG_CONFIG_SYSROOT_DIR:${SYSROOT}
export PKG_CONFIG_ALLOW_CROSS=1

OPENSSL_DIR=$SCRIPTPATH/ssl_arm/ cargo build --target=armv7-unknown-linux-gnueabihf --release
