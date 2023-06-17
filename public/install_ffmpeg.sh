#!/bin/sh

install_with_apt() {
  apt-get install --no-install-recommends ffmpeg -y
}

case "$(uname -m)" in
  x86_64|amd64)
    arch="64"
    ;;
  aarch64|arm64)
    arch="arm64"
    ;;
  *)
    install_with_apt
    ;;
esac

install_without_apt() {
  ffmpeg_filename="ffmpeg-master-latest-linux${arch}-gpl"
  wget -c "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/$ffmpeg_filename.tar.xz" -O - | tar -xJf - -C /usr/local/bin $ffmpeg_filename/bin/ffmpeg
  rm -f $ffmpeg_filename.tar.xz
}

install_without_apt
