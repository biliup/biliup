FROM python:3.9.13-slim
LABEL containers.image.authors="hello"
LABEL version="1.0"
ENV TZ=Asia/Shanghai
VOLUME /opt
COPY requirements.txt requirements.txt
RUN  apt-get -y update
RUN  apt-get -y install ffmpeg
RUN pip install --upgrade pip
RUN pip install -r requirements.txt
WORKDIR /src



