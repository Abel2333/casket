FROM rust:1.89

RUN apt-get update && apt-get install -y \
  nodejs \
  npm \
  git \
  pkg-config \
  libssl-dev \
  libsqlite3-dev \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*
