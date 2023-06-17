#!/bin/sh

install_with_apt() {
  apt-get install --no-install-recommends ffmpeg -y
}

use_apt=false

case "$(uname -m)" in
  x86_64|amd64)
    arch="64"
    ;;
  aarch64|arm64)
    arch="arm64"
    ;;
  *)
    use_apt=true
    ;;
esac

install_without_apt() {
  ffmpeg_filename="ffmpeg-master-latest-linux${arch}-gpl"
  wget -c "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/$ffmpeg_filename.tar.xz"
  echo $ffmpeg_filename.tar.xz
  tar -xvf $ffmpeg_filename.tar.xz $ffmpeg_filename/bin/ffmpeg
  mv $ffmpeg_filename/bin/ffmpeg /usr/local/bin/
  rm -rf $ffmpeg_filename*
}

if $use_apt; then
  install_with_apt
else
  install_without_apt
fi