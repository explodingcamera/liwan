FROM busybox AS downloader
ARG TAR_URL_AMD64
ARG TAR_URL_ARM64

WORKDIR /app

RUN [ "${TARGETPLATFORM}" = "linux/arm64" ] && export TAR_URL=${TAR_URL_ARM64} || export TAR_URL=${TAR_URL_AMD64} \
    && curl -fsSL $TAR_URL -o /tmp/package.tar.gz \
    && tar -xzf /tmp/package.tar.gz -C /app \
    && chmod +x /app/liwan

FROM scratch

ENV LIWAN_CONFIG=/app/liwan.config.toml
ENV LIWAN_DATA_DIR=/data

COPY --from=downloader /app/liwan /liwan
ENTRYPOINT ["/liwan"]
EXPOSE 9042