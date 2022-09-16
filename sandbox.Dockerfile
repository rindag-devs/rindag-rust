FROM criyle/executorserver:latest

RUN apt update && apt install -y --no-install-recommends gcc g++ && rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["./executorserver"]
