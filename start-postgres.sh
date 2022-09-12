#!/usr/bin/env bash

docker run -d \
	--name rindag-postgres \
  -e POSTGRES_USER=root \
	-e POSTGRES_PASSWORD=root \
  -e POSTGRES_DB=rindag \
	-e PGDATA=/var/lib/postgresql/data/pgdata \
	-v /var/lib/postgresql/data:/var/lib/postgresql/data \
  -p 5432:5432 \
  --rm \
	postgres:14
