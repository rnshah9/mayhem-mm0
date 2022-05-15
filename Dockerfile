# Build Stage
FROM --platform=linux/amd64 ubuntu:20.04 as builder
RUN apt-get update

RUN DEBIAN_FRONTEND=noninteractive apt-get install  -y clang

ADD . /mm0
WORKDIR /mm0/mm0-c

RUN clang main.c -o mm0-c -D NO_PARSER

FROM ubuntu:20.04

COPY --from=builder /mm0/mm0-c/mm0-c .
