FROM rust:slim

RUN useradd -ms /bin/bash myuser

USER myuser

ENV RUSTUP_UPDATE_ROOT="https://mirrors.tuna.tsinghua.edu.cn/rustup/rustup"
ENV RUSTUP_DIST_SERVER="https://mirrors.tuna.tsinghua.edu.cn/rustup"

RUN rustup component add rustfmt