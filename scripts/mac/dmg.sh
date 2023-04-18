#!/bin/bash

set -e

APP_NAME=dgb
MACOS_BIN_NAME=dgb
MACOS_APP_NAME=DigitalesGrundbuch
MACOS_APP_DIR=$MACOS_APP_NAME.app

mkdir -p macbuild
cd macbuild
echo "Creating app directory structure"
rm -rf $MACOS_APP_NAME
rm -rf $MACOS_APP_DIR
mkdir -p $MACOS_APP_DIR/Contents/MacOS

cargo build # --release

echo "pwd"
pwd

echo "Copying binary"
MACOS_APP_BIN=$MACOS_APP_DIR/Contents/MacOS/$MACOS_BIN_NAME
cp ../target/debug/$APP_NAME $MACOS_APP_BIN
# cp ../target/release/$APP_NAME $MACOS_APP_BIN

echo "Copying launcher"
cp ../scripts/mac/macos_launch.sh $MACOS_APP_DIR/Contents/MacOS/$MACOS_APP_NAME

echo "Copying Icon"
mkdir -p $MACOS_APP_DIR/Contents/Resources
cp ../scripts/mac/Info.plist $MACOS_APP_DIR/Contents/
cp ../scripts/mac/dgb.icns $MACOS_APP_DIR/Contents/Resources/

echo "Creating dmg"
mkdir -p $MACOS_APP_NAME
cp -r $MACOS_APP_DIR $MACOS_APP_NAME/
rm -rf $MACOS_APP_NAME/.Trashes
ln -s /Applications $MACOS_APP_NAME/Applications

FULL_NAME=$MACOS_APP_NAME

hdiutil create $FULL_NAME.dmg -srcfolder `pwd`/$MACOS_APP_NAME -ov
rm -rf $MACOS_APP_NAME