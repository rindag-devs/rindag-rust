#!/usr/bin/env bash

docker run -d \
  --name rindag-redis \
  --rm \
  -p 6379:6379 \
  redis:7-alpine
