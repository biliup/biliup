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
FROM python:3.9-slim as biliup
ENV TZ=Asia/Shanghai
EXPOSE 19159/tcp
VOLUME /opt

RUN \
  set -eux; \
#  apk update && \
    # save list of currently installed packages for later so we can clean up
  savedAptMark="$(apt-mark showmanual)"; \
  apt-get update; \
#  apk add --no-cache --virtual .build-deps git curl gcc g++ && \
#  apk add --no-cache ffmpeg musl-dev libffi-dev zlib-dev jpeg-dev ca-certificates && \
  apt-get install -y --no-install-recommends ffmpeg git g++; \
  git clone --depth 1 https://github.com/ForgQi/biliup.git && \
  cd biliup && \
  pip3 install --no-cache-dir quickjs && \
  pip3 install -e . && \
  # Clean up \
  apt-mark auto '.*' > /dev/null; \
  apt-mark manual ffmpeg; \
  [ -z "$savedAptMark" ] || apt-mark manual $savedAptMark; \
  find /usr/local -type f -executable -exec ldd '{}' ';' \
     | awk '/=>/ { print $(NF-1) }' \
     | sort -u \
     | xargs -r dpkg-query --search \
     | cut -d: -f1 \
     | sort -u \
     | xargs -r apt-mark manual \
     ; \
  apt-get purge -y --auto-remove -o APT::AutoRemove::RecommendsImportant=false; \
  rm -rf \
    /tmp/* \
    /usr/share/doc/* \
    /var/cache/* \
    /var/lib/apt/lists/* \
    /var/tmp/* && \
  #  apk del --purge .build-deps && \
#  rm -rf /var/cache/apk/* && \
  rm -rf /var/log/*

COPY --from=webui /biliup/biliup/web/public/ /biliup/biliup/web/public/
WORKDIR /opt

ENTRYPOINT ["biliup"]
