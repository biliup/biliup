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
	apt-get install -y --no-install-recommends python3-pip g++ patchelf; \
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
	# 固定 FFmpeg 到确定版本并校验 SHA-256：
	# latest 是滚动 tag，资产每日重建，既不可复现也无法防篡改。
	# 只能固定到「每月最后一天」的 autobuild：BtbN 长期保留月末构建，
	# 其余每日构建约两周后删除，固定到它们会让镜像构建 404。
	url='https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-08-31-13-27/ffmpeg-n8.1.2-50-g1a748fe2cd-'; \
	case "$arch" in \
		'amd64') \
			url="${url}linux64-gpl-8.1.tar.xz"; \
			sha256='c733b4b2951e5957e15505f788b2c65a7a41b6da4b289e295852cc38079b4d2b'; \
		;; \
		'arm64') \
			url="${url}linuxarm64-gpl-8.1.tar.xz"; \
			sha256='ae5da4f51b9052390f414005f8ab26c1eed1268f327cce7cb79aa076b29bd66e'; \
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
		echo "$sha256  ffmpeg.tar.xz" | sha256sum -c -; \
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
# 容器内必须监听 0.0.0.0，否则宿主机的端口映射无法转发进容器。
# CLI 默认的 127.0.0.1 只适用于直接在本机运行的场景。
CMD ["server", "--bind", "0.0.0.0", "--auth"]
