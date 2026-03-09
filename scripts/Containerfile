FROM gcr.io/distroless/cc-debian13:nonroot
ARG TARGETARCH

ENV LIWAN_CONFIG=/liwan.config.toml
ENV LIWAN_DATA_DIR=/data

USER 1000:1000

COPY ./dist/${TARGETARCH:+amd64/}liwan /liwan

EXPOSE 9042
STOPSIGNAL SIGINT
ENTRYPOINT ["/liwan"]