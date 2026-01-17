FROM gcr.io/distroless/cc-debian13:nonroot

ENV LIWAN_CONFIG=/liwan.config.toml
ENV LIWAN_DATA_DIR=/data

COPY --from=buildx /dist/${TARGETARCH:+amd64/}liwan /liwan

EXPOSE 9042
STOPSIGNAL SIGINT
ENTRYPOINT ["/liwan"]