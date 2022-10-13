#!/bin/bash

set -eu

VERSION=`head -n 3 Cargo.toml | tail -n 1 | sed -r 's/^.*\"(.*)\"$/\1/'`

echo "Deploying v$VERSION..."

docker login

build_tag_push () {
    docker build --build-arg COMPONENT=$1 -t negy-$1 .
    docker tag negy-$1 tbrand/negy-$1:latest
    docker tag negy-$1 tbrand/negy-$1:$VERSION
    docker push tbrand/negy-$1:latest
    docker push tbrand/negy-$1:$VERSION
}

build_tag_push "node-pool"
build_tag_push "node"
build_tag_push "gateway"
