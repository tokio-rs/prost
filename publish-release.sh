#!/bin/bash

# Script which automates publishing a crates.io release of the prost crates.

set -ex

if [ "$#" -ne 0 ]
then
  echo "Usage: $0"
  exit 1
fi

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

CRATES=( \
  "prost-derive" \
  "." \
  "prost-build" \
  "prost-types" \
)

for CRATE in "${CRATES[@]}"; do
  pushd $CRATE
  cargo publish
  popd
done
