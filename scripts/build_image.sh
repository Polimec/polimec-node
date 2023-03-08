#!/bin/bash

# Build Container Image from Dockerfile
# Usage: ./build_image.sh <version> <path to Dockerfile/Dockerfile> <package>
# Example: ./build_image.sh latest ./Dockerfile polimec-standalone-node

# Check if tag is passed
if [ -z "$1" ]
then
    echo "Please pass tag as first argument"
    echo "Example: ./build_image.sh latest ../Dockerfile polimec-standalone-node"
    exit 1
fi

# Check if Dockerfile is passed
if [ -z "$2" ]
then
    echo "Please pass path to Dockerfile as second argument"
    echo "Example: ./build_image.sh latest ../Dockerfile polimec-standalone-node"
    exit 1
fi

# Check if Dockerfile exists
if [ ! -f "$2" ]
then
    echo "Dockerfile does not exist"
    exit 1
fi

# Check if package is passed
if [ -z "$3" ]
then
    echo "Please pass package as third argument"
    echo "Example: ./build_image.sh latest ../Dockerfile polimec-standalone-node"
    exit 1
fi

# Build Docker Image
docker build -t "docker.io/polimec/$3:$1" -f $2 --build-arg PACKAGE=$3 .
exit 0
