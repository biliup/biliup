# Build biliup's web-ui
FROM node:16-alpine as webui

RUN \
  set -eux && \
  apk add --no-cache git && \
  git clone --depth 1 https://github.com/ForgQi/biliup.git && \
  cd biliup && \
  npm install && \
  npm run build

# Deploy Biliup
FROM python:3.9-alpine as biliup

EXPOSE 19159/tcp
VOLUME /opt

RUN \
  set -eux && \
  apk update && \
  apk add --no-cache --virtual .build-deps git curl gcc g++ && \
  apk add --no-cache ffmpeg musl-dev libffi-dev zlib-dev jpeg-dev ca-certificates && \
  git clone --depth 1 https://github.com/ForgQi/biliup.git && \
  cd biliup && \
  pip3 install --no-cache-dir quickjs && \
  pip3 install -e . && \
  # Clean up
  apk del --purge .build-deps && \
  rm -rf /var/cache/apk/* && \
  rm -rf /tmp/* && \
  rm -rf /var/log/*

COPY --from=webui /biliup/biliup/web/public/ /biliup/biliup/web/public/
WORKDIR /opt

ENTRYPOINT ["biliup", "--http"]
