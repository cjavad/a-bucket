FROM rust:1.71-alpine as builder
WORKDIR /app
COPY . .
RUN apk add --no-cache musl-dev openssl-dev
RUN cargo build --release

FROM alpine:latest

RUN apk add --no-cache libssl1.1
WORKDIR /app
COPY --from=builder /app/target/release/a-bucket .
EXPOSE 8000
CMD [ "./a-bucket" ]