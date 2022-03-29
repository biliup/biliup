FROM python:3.9-alpine3.14

VOLUME /opt

RUN \
  set -eux && \
  apk update && \
  apk add --no-cache --virtual .build-deps git curl gcc g++ && \
  apk add --no-cache ffmpeg musl-dev libffi-dev zlib-dev jpeg-dev ca-certificates && \
  git clone --depth 1 https://github.com/ForgQi/biliup.git && \
  cd biliup && \
  pip3 install --no-cache-dir quickjs && \
  pip3 install --no-cache-dir -r requirements.txt && \
  pip3 install -e . && \
  apk del .build-deps

WORKDIR /opt
EXPOSE 19159/tcp
ENTRYPOINT ["biliup", "--http"]
