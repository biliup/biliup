FROM jrottenberg/ffmpeg
#VOLUME /opt/data
RUN apt-get update && apt-get install -y locales && rm -rf /var/lib/apt/lists/* \
    && localedef -i en_US -c -f UTF-8 -A /usr/share/locale/locale.alias en_US.UTF-8
ENV LANG en_US.utf8
RUN export DEBIAN_FRONTEND=noninteractive \
  && apt-get update \
  && apt-get install -y curl \
  && apt-get install -y python3-pip \
  && apt-get install \
    unzip \
  && \
  DL=https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb \
  && curl -sL "$DL" > /tmp/chrome.deb \
  && apt install --no-install-recommends --no-install-suggests -y \
    /tmp/chrome.deb \
  && CHROMIUM_FLAGS='--no-sandbox --disable-dev-shm-usage' \
  # Patch Chrome launch script and append CHROMIUM_FLAGS to the last line:
  && sed -i '${s/$/'" $CHROMIUM_FLAGS"'/}' /opt/google/chrome/google-chrome \
  && BASE_URL=https://chromedriver.storage.googleapis.com \
  && VERSION=$(curl -sL "$BASE_URL/LATEST_RELEASE") \
  && curl -sL "$BASE_URL/$VERSION/chromedriver_linux64.zip" -o /tmp/driver.zip \
  && unzip /tmp/driver.zip \
  && chmod 755 chromedriver \
  && mv chromedriver /usr/local/bin/ \
  # Remove obsolete files:
  && apt-get autoremove --purge -y \
    unzip \
  && apt-get clean \
  && rm -rf \
    /tmp/* \
    /usr/share/doc/* \
    /var/cache/* \
    /var/lib/apt/lists/* \
    /var/tmp/*

COPY requirements.txt /opt/
RUN cd /opt \
    && pip3 install -r requirements.txt
#USER webdriver
COPY common /opt/common
COPY engine /opt/engine
COPY ykdl /opt/ykdl
COPY Bilibili.py /opt/
RUN  chmod 755 /opt/Bilibili.py
COPY ["config(demo).yaml", "/opt/config.yaml"]

WORKDIR /opt
ENTRYPOINT ["./Bilibili.py"]

EXPOSE 9515/tcp
