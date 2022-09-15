#!/usr/bin/env bash

docker run -d \
	--name rindag-judge-server \
  -it \
  --rm \
  --privileged \
  --shm-size=1g \
  -p 5050:5050 \
  rindag-judge-server \
  -file-timeout 30m
