#!/bin/bash

set -ex

if [ "$#" -ne 1 ]
then
  echo "Usage: $0 <protobuf-version>"
  exit 1
fi

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
VERSION="$1"
TEMPDIR=$(mktemp -d "protobuf-$VERSION-XXX")
ARCH="linux-x86_64"

mkdir "$TEMPDIR/$ARCH"
curl --proto '=https' --tlsv1.2 -sSfL \
  "https://github.com/protocolbuffers/protobuf/releases/download/v$VERSION/protoc-$VERSION-$ARCH.zip" \
  -o "$TEMPDIR/$ARCH/protoc.zip"

unzip "$TEMPDIR/$ARCH/protoc.zip" -d "$TEMPDIR/$ARCH"

# Update the include directory
rm -rf "$DIR/include"
mv "$TEMPDIR/linux-x86_64/include" "$DIR/include/"


rm -rf $TEMPDIR
cd "$DIR/protobuf"
git checkout "v$VERSION"
cd $DIR

echo "third-party protobuf items updated to v$VERSION"