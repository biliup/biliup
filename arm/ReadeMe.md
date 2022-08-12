#1. docker build -t biliup_arm .
#2. git clone https://github.com/javaoraspx/biliup.git #如果被合并,请更换原作者仓库
#3. docker run -it -v $PWD/biliup:/src  biliup_arm:latest python3 -m biliup --config /src/arm/configtest.yaml #此处仅可以进行录制,如需web管理还需增加node 环境