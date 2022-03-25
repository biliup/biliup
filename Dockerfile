FROM python:3.9-alpine3.14

VOLUME /opt

RUN \
  set -eux && \
  apk update --no-cache && \
  apk add --no-cache --virtual .build-deps git gcc g++ && \
  apk add --no-cache ffmpeg musl-dev libffi-dev zlib-dev jpeg-dev ca-certificates && \
  git clone --depth 1 https://github.com/biliup/biliup.git && \
  # git clone --depth 1 -b dev https://github.com/xxxxuanran/biliup.git && \
  cd biliup && \
  pip3 install --no-cache-dir quickjs && \
  pip3 install --no-cache-dir -e . && \
  apk del .build-deps

WORKDIR /opt

ENTRYPOINT ["biliup"]
