FROM debian:13-slim AS downloader
ARG TAR_URL_AMD64
ARG TAR_URL_ARM64
ARG TARGETPLATFORM

RUN apt-get update && apt-get install -y curl tar
RUN echo "Downloading liwan for ${TARGETPLATFORM}..." \
    && TAR_URL=$(if [ "${TARGETPLATFORM}" = "linux/arm64" ]; then echo ${TAR_URL_ARM64}; else echo ${TAR_URL_AMD64}; fi) \
    && curl -fsSL $TAR_URL -o /tmp/package.tar.gz \
    && mkdir -p /app \
    && tar -xzf /tmp/package.tar.gz -C /app \
    && chmod +x /app/liwan

FROM alpine:3

ENV LIWAN_CONFIG=/liwan.config.toml
ENV LIWAN_DATA_DIR=/data

COPY --from=downloader /app/liwan /liwan
ENTRYPOINT ["/liwan"]
EXPOSE 9042
STOPSIGNAL SIGINT