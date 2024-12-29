# Ext4

## Build

使用docker
```bash
# 创建镜像
docker build -f Dockerfile -t ext4_rs .
# 创建容器
docker run -it -v $(pwd):/ext4_rs -w /ext4_rs --name ext4_rs --network host ext4_rs bash
# 进入容器
docker exec -it ext4_rs bash
```

## Usage

实现了对于文件和目录的创建/读取

具体实现用法可以参考tests