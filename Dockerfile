FROM rust:1-alpine as builder
RUN apk --no-cache add curl make libc-dev
WORKDIR /usr/src/p2p_chat
COPY . .
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
RUN make build-release

FROM alpine:3.7 as production
COPY --from=builder /usr/src/p2p_chat/static /var/www
ENV P2P_STATIC_FILES /var/www
COPY --from=builder /usr/src/p2p_chat/target/release/server /usr/local/bin/server_p2p
CMD ["server_p2p"]
