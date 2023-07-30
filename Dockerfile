# Build biliup's web-ui
FROM node:16-alpine as webui

COPY . /biliup
RUN \
  set -eux && \
  cd /biliup && \
  npm install && \
  npm run build

# Deploy Biliup
FROM python:3.9-slim as biliup
ENV TZ=Asia/Shanghai
EXPOSE 19159/tcp
VOLUME /opt

COPY . /biliup
RUN \
  set -eux; \
  savedAptMark="$(apt-mark showmanual)"; \
  apt-get update; \
  apt-get install -y --no-install-recommends ffmpeg g++; \
  cd /biliup && \
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
