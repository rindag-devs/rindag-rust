#!/usr/bin/env bash

docker run -d \
	--name rindag-sandbox \
  -it \
  --rm \
  --privileged \
  --shm-size=1g \
  -p 5050:5050 \
  -p 5051:5051 \
  rindag-sandbox \
  -enable-grpc
