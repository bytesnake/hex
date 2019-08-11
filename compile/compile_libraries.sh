#!/bin/bash
PATH_OUT=$(pwd)"/compile/libraries/"
COMPILER=$HOST-gcc

mkdir -p $PATH_OUT

set -ex

if [ ! -d "$PATH_OUT/alsa/" ] ; then
    if [ ! -d "alsa-lib" ] ; then
        git clone git://git.alsa-project.org/alsa-lib.git alsa-lib
    fi

    cd alsa-lib

    libtoolize --force --copy --automake
    aclocal
    autoheader
    automake --foreign --copy --add-missing
    autoconf

    ./configure --host=$HOST --prefix=$PATH_OUT/alsa/ --enable-shared && make --no-print-directory && make install -s

    cd ..
fi


if [ ! -d "$PATH_OUT/opus/" ] ; then
    if [ ! -d "opus-1.3.1" ] ; then
        wget https://archive.mozilla.org/pub/opus/opus-1.3.1.tar.gz
        tar -xzvf opus-1.3.1.tar.gz
    fi

    cd opus-1.3.1
    ./configure --host=$HOST --prefix=$PATH_OUT/opus/ --enable-shared && make -s && make install -s

    cd ..
fi

if [ ! -d "$PATH_OUT/openssl/" ] ; then
    if [ ! -d "openssl-OpenSSL_1_1_1c" ] ; then
        wget https://github.com/openssl/openssl/archive/OpenSSL_1_1_1c.tar.gz
        tar -xzvf OpenSSL_1_1_1c.tar.gz
    fi

    cd openssl-OpenSSL_1_1_1c

    if [ "$HOST" = "x86_64-pc-linux-gnu" ] ; then
        ./Configure $SSL_HOST --prefix=$PATH_OUT/openssl/ --openssldir=$PATH_OUT/openssl/ --cross-compile-prefix=$HOST-gcc- shared 
    else 
        ./Configure $SSL_HOST --prefix=$PATH_OUT/openssl/ --openssldir=$PATH_OUT/openssl/ --cross-compile-prefix=$HOST- shared 
    fi

    make CC=$COMPILER
    make install

    cd ..
fi

