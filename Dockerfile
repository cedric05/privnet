FROM rust:alpine
RUN apk add musl-dev
ADD . /app/
WORKDIR /app/
RUN cargo build --release
FROM alpine
COPY --from=0 /app/target/release/privnet /usr/bin/privnet
ENTRYPOINT ["/usr/bin/privnet"]
CMD ["server"]
