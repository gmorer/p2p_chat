FROM rust:1-alpine3.12 as builder
RUN apk --no-cache add curl make libc-dev
WORKDIR /usr/src/p2pchat
COPY . .
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh 
RUN make build-release

FROM alpine:3.7 as production
COPY --from=builder /usr/src/p2pchat/static /var/www
ENV P2P_STATIC_FILES /var/www
COPY --from=builder /usr/src/p2pchat/target/release/server /usr/local/bin/server_p2p
CMD ["server_p2p"]
