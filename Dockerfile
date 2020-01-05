FROM lancern/libseccomp:2.4-stretch AS libseccomp
FROM rust:1.40-stretch AS build
WORKDIR /deps
COPY --from=libseccomp /libseccomp ./libseccomp

WORKDIR /app
COPY ./ ./
ENV LIBSECCOMP_LIB_PATH=/deps/libseccomp/lib LIBSECCOMP_LIB_TYPE=static
RUN cargo build --release

FROM debian:stretch-slim AS runtime
WORKDIR /app
COPY --from=build /app/target/release ./
ENTRYPOINT ["/app/seccompbench"]
