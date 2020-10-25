FROM jrottenberg/ffmpeg
#VOLUME /opt/data
RUN export DEBIAN_FRONTEND=noninteractive \
  && apt-get update \
  && apt install build-essential -y \
  && apt install libncurses5-dev libgdbm-dev libnss3-dev libssl-dev libreadline-dev libffi-dev -y \
  && apt install wget -y \
  && apt install openssl -y \
  && apt install curl -y \
  && apt install libsqlite3-dev -y \
  && wget https://www.python.org/ftp/python/3.7.3/Python-3.7.3.tgz \
  && tar -xvf Python-3.7.3.tgz \
  && cd Python-3.7.3 \
  && ./configure --enable-loadable-sqlite-extensions \
  && make \
  && make install \
  && ln -s /usr/local/bin/pip3 /usr/bin/pip3 \
  && ln -s /usr/local/bin/python3 /usr/bin/python3 \
#  && apt-get install -y python3-pip \
  && apt-get install -y git \
  && apt-get install -y zip \
  && apt-get install -y nodejs \
#  && apt-get install -y unzip \
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
  && apt-get install -y locales \
  && localedef -i en_US -c -f UTF-8 -A /usr/share/locale/locale.alias en_US.UTF-8 \
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

ENV LANG en_US.utf8

COPY requirements.txt /opt/
RUN cd /opt \
    && pip3 install -r requirements.txt
#USER webdriver
COPY common /opt/common
COPY engine /opt/engine
COPY Bilibili.py /opt/
RUN chmod 755 /opt/Bilibili.py
COPY ["config(demo).yaml", "/opt/config.yaml"]

WORKDIR /opt
ENTRYPOINT ["./Bilibili.py"]

#EXPOSE 9515/tcp
