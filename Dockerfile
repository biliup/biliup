# Build biliup's web-ui
FROM node:lts as webui
ARG repo_url
ENV REPO_URL $repo_url

RUN set -eux; \
	git clone --depth 1 $REPO_URL; \
	cd biliup; \
	npm install; \
	npm run build

# Deploy Biliup
FROM python:3.12-slim as biliup
ARG repo_url
ENV REPO_URL $repo_url
ENV TZ=Asia/Shanghai
EXPOSE 19159/tcp
VOLUME /opt

RUN set -eux; \
	\
	savedAptMark="$(apt-mark showmanual)"; \
	useApt=false; \
	apt-get update; \
	apt-get install -y --no-install-recommends \
		wget \
		xz-utils \
	; \
	apt-mark auto '.*' > /dev/null; \
	\
	arch="$(dpkg --print-architecture)"; arch="${arch##*-}"; \
	url='https://github.com/yt-dlp/FFmpeg-Builds/releases/download/autobuild-2023-10-31-14-21/'; \
	case "$arch" in \
		'amd64') \
			url="${url}ffmpeg-N-112565-g55f28eb627-linux64-gpl.tar.xz"; \
		;; \
		'arm64') \
			url="${url}ffmpeg-N-112565-g55f28eb627-linuxarm64-gpl.tar.xz"; \
		;; \
		*) \
			useApt=true; \
		;; \
	esac; \
	\
	if [ "$useApt" = true ] ; then \
		apt-get install -y --no-install-recommends \
			ffmpeg \
		; \
	else \
		wget -O ffmpeg.tar.xz "$url" --progress=dot:giga; \
		tar -xJf ffmpeg.tar.xz -C /usr/local --strip-components=1; \
		rm -rf \
			/usr/local/doc \
			/usr/local/man; \
		rm -rf \
			ffmpeg*; \
	fi; \
	\
	# Clean up \
	[ -z "$savedAptMark" ] || apt-mark manual $savedAptMark; \
	apt-get purge -y --auto-remove -o APT::AutoRemove::RecommendsImportant=false; \
	rm -rf \
		/tmp/* \
		/usr/share/doc/* \
		/var/cache/* \
		/var/lib/apt/lists/* \
		/var/tmp/* \
		/var/log/*

RUN set -eux; \
	savedAptMark="$(apt-mark showmanual)"; \
	apt-get update; \
	apt-get install -y --no-install-recommends git g++; \
	git clone --depth 1 $REPO_URL && \
	cd biliup && \
	pip3 install --no-cache-dir quickjs && \
	pip3 install -e . && \
	\
	# Clean up \
	apt-mark auto '.*' > /dev/null; \
	[ -z "$savedAptMark" ] || apt-mark manual $savedAptMark; \
	apt-get purge -y --auto-remove -o APT::AutoRemove::RecommendsImportant=false; \
	rm -rf \
		/tmp/* \
		/usr/share/doc/* \
		/var/cache/* \
		/var/lib/apt/lists/* \
		/var/tmp/* \
		/var/log/*; \
	\
	printf '#!/bin/sh\nparams="$*"\nwhile true; do\nbiliup $params\nsleep 1\necho "Script restart"\ndone\n' > run_biliup.sh; \
	chmod a+x run_biliup.sh

COPY --from=webui /biliup/biliup/web/public/ /biliup/biliup/web/public/
WORKDIR /opt

ENTRYPOINT ["/biliup/run_biliup.sh"]
