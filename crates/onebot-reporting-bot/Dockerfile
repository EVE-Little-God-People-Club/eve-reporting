FROM rust:1.93.0 AS builder

RUN apt-get update && \
	apt-get install -y --no-install-recommends \
		musl-tools \
		gcc \
		g++ \
		make \
		pkg-config \
		ca-certificates && \
	rm -rf /var/lib/apt/lists/*

WORKDIR /builder
COPY . .


RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:3.23.3

RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY --from=builder /builder/target/x86_64-unknown-linux-musl/release/onebot-reporting-bot /app/onebot-reporting-bot

ENV ONEBOT_URL="<YOUR URL>"
ENV ONEBOT_KEY="<YOUR KEY>"
ENV MESSAGE_TARGET="<YOUR QQ ID>"
ENV WARN_SPACING="<WARN SPACING>"

EXPOSE 8080/tcp
ENTRYPOINT ["./onebot-reporting-bot"]

