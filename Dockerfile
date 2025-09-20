# Build biliup's web-ui
FROM node:lts AS webui-builder
ARG repo_url=https://github.com/biliup/biliup
ARG branch_name=master

COPY . /biliup

RUN set -eux; \
	\
	if [ ! -f /biliup/biliup.spec ]; then \
	rm -rf /biliup; \
	git clone --depth 1 --branch "$branch_name" "$repo_url" /biliup; \
	fi;

WORKDIR /biliup

RUN set -eux; \
	npm install; \
	npm run build;


# Build biliup's python wheel
FROM rust:latest AS wheel-builder
ARG repo_url=https://github.com/biliup/biliup
ARG branch_name=master

COPY . /biliup

RUN set -eux; \
	\
	apt-get update; \
	apt-get install -y --no-install-recommends python3-pip g++; \
	pip3 install maturin --break-system-packages; \
	if [ ! -f /biliup/biliup.spec ]; then \
	rm -rf /biliup; \
	git clone --depth 1 --branch "$branch_name" "$repo_url" /biliup; \
	fi;

COPY --from=webui-builder /biliup/out /biliup/out

WORKDIR /biliup

RUN set -eux; \
	maturin build --release;


# Deploy Biliup
FROM python:3.13-slim AS biliup

ENV TZ="Asia/Shanghai"
ENV LANG="C.UTF-8"
ENV LANGUAGE="C.UTF-8"
ENV LC_ALL="C.UTF-8"
EXPOSE 19159/tcp
VOLUME /opt

# 需要遵守 wheel 文件名规范
COPY --from=wheel-builder /biliup/target/wheels/* /tmp/

RUN set -eux; \
	\
	whl=$(ls /tmp/biliup*.whl); \
	pip3 install --no-cache-dir "$whl"; \
	# pip3 install --no-cache-dir "$whl[quickjs]"; \
	pip3 cache purge; \
	rm -rf /tmp/*;

RUN set -eux; \
	\
	savedAptMark="$(apt-mark showmanual)"; \
	useApt=false; \
	apt-get update; \
	apt-get install -y --no-install-recommends \
		wget \
		curl \
		xz-utils \
		g++ \
	; \
	apt-mark auto '.*' > /dev/null; \
	apt-mark manual curl wget; \
	\
	arch="$(dpkg --print-architecture)"; arch="${arch##*-}"; \
	url='https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n8.0-latest-'; \
	case "$arch" in \
		'amd64') \
			url="${url}linux64-gpl-8.0.tar.xz"; \
		;; \
		'arm64') \
			url="${url}linuxarm64-gpl-8.0.tar.xz"; \
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
			/usr/local/bin/ffprobe \
			/usr/local/bin/ffplay; \
		rm -rf \
			ffmpeg*; \
		chmod a+x /usr/local/* ; \
	fi; \
	\
	# 安装 quickjs 需要 g++
	pip3 install --no-cache-dir quickjs; \
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
		/var/log/* \
	;

WORKDIR /opt

ENTRYPOINT ["biliup"]
