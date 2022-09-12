#!/usr/bin/env bash

docker run -d \
  -p 9000:9000 \
  -p 9001:9001 \
  --rm \
  --name rindag-minio \
	-v /var/lib/rindag/minio:/data \
  -e "MINIO_ROOT_USER=minio" \
  -e "MINIO_ROOT_PASSWORD=minio123" \
  quay.io/minio/minio server /data --console-address ":9001"
