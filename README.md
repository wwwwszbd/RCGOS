# RCGOS
My RISC-V Operating System

# 配置环境
# WSL2 手动配置 rCore 开发环境

## 环境概览

- **宿主系统**: Windows 10/11
- **WSL2 系统**: Ubuntu 20.04 LTS
- **目标架构**: RISC-V 64位

## 目录结构设计

```
~/rCore-dev/                    # 开发根目录
├── bin/                        # 自定义脚本和工具
├── downloads/                  # 下载的文件
├── qemu-7.0.0/                # QEMU 源码和构建
│   ├── build/                 # QEMU 构建目录
│   └── qemu-7.0.0.tar.xz      # 下载的源码包
├── riscv-toolchain/           # RISC-V 工具链
├── projects/                  # 项目目录
│   ├── rCore-Tutorial-v3/     # 官方教程（只读，用于参考）
│   ├── my-rCore-OS/          # 你的操作系统项目（主要工作区）
│   └── experiments/           # 实验和小项目
├── config/                    # 配置文件
├── logs/                      # 日志文件
└── README.md                  # 环境说明文档
```

## 配置步骤

### 步骤 1: 初始化和创建目录结构

```bash
# 1. 在 WSL2 Ubuntu 中打开终端
# 2. 创建基础目录结构
cd ~
mkdir -p rCore-dev/{bin,downloads,projects,config,logs}

# 创建项目子目录
mkdir -p rCore-dev/projects/{rCore-Tutorial-v3,my-rCore-OS,experiments}
```

### 步骤 2: 更新系统和安装基础依赖

```bash
# 更新包列表
sudo apt update && sudo apt upgrade -y

# 安装基础开发工具
sudo apt install -y \
    git curl wget \
    build-essential \
    gcc g++ make cmake \
    pkg-config \
    python3 python3-pip \
    vim nano \
    tree htop tmux
```

### 步骤 3: 配置 Rust 开发环境

```bash
# 创建 Rust 安装配置脚本
cat > ~/rCore-dev/bin/setup-rust.sh << 'EOF'
#!/bin/bash
echo "Setting up Rust development environment..."

# 设置 Rust 镜像源
export RUSTUP_DIST_SERVER=https://mirrors.tuna.tsinghua.edu.cn/rustup
export RUSTUP_UPDATE_ROOT=https://mirrors.tuna.tsinghua.edu.cn/rustup/rustup

# 安装 rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 设置环境变量
source $HOME/.cargo/env

# 配置 cargo 镜像源
mkdir -p $HOME/.cargo
cat > $HOME/.cargo/config << 'CONFIG'
[source.crates-io]
replace-with = 'tuna'

[source.tuna]
registry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"

[net]
git-fetch-with-cli = true
CONFIG

# 安装 nightly 版本并设置为默认
rustup install nightly
rustup default nightly

# 添加 RISC-V 目标
rustup target add riscv64gc-unknown-none-elf

# 安装其他 Rust 工具
cargo install cargo-binutils
rustup component add llvm-tools-preview
rustup component add rust-src

echo "Rust setup complete!"
rustc --version
EOF

# 赋予执行权限并运行
chmod +x ~/rCore-dev/bin/setup-rust.sh
~/rCore-dev/bin/setup-rust.sh
```

### 步骤 4: 安装 QEMU 7.0+

```bash
# 创建 QEMU 安装脚本
cat > ~/rCore-dev/bin/setup-qemu.sh << 'EOF'
#!/bin/bash
echo "Setting up QEMU..."

cd ~/rCore-dev/downloads

# 下载 QEMU 7.0.0
if [ ! -f qemu-7.0.0.tar.xz ]; then
    wget https://download.qemu.org/qemu-7.0.0.tar.xz
fi

# 解压
tar -xf qemu-7.0.0.tar.xz

# 移动源码
mv qemu-7.0.0 ~/rCore-dev/

# 安装编译依赖
sudo apt install -y \
    autoconf automake autotools-dev \
    libmpc-dev libmpfr-dev libgmp-dev \
    gawk build-essential bison flex texinfo gperf \
    libtool patchutils bc zlib1g-dev libexpat-dev \
    pkg-config libglib2.0-dev libpixman-1-dev \
    git ninja-build

# 编译 QEMU
cd ~/rCore-dev/qemu-7.0.0
mkdir -p build
cd build
../configure --target-list=riscv64-softmmu,riscv64-linux-user
make -j$(nproc)

# 添加到 PATH
echo 'export PATH="$HOME/rCore-dev/qemu-7.0.0/build:$PATH"' >> ~/.bashrc
source ~/.bashrc

echo "QEMU setup complete!"
qemu-system-riscv64 --version
EOF

# 安装 QEMU
chmod +x ~/rCore-dev/bin/setup-qemu.sh
~/rCore-dev/bin/setup-qemu.sh
```

cd ~/rCore-dev/downloads

# 检查文件
ls -lh riscv64-unknown-elf-gcc-8.3.0-2020.04.1-x86_64-linux-ubuntu14.tar

# 解压到 riscv-toolchain 目录
mkdir -p ~/rCore-dev/riscv-toolchain
tar -xvf riscv64-unknown-elf-gcc-8.3.0-2020.04.1-x86_64-linux-ubuntu14.tar -C ~/rCore-dev/riscv-toolchain --strip-components=1

# 查看解压后的内容
ls -la ~/rCore-dev/riscv-toolchain/
步骤 2: 配置环境变量
bash
# 添加到 .bashrc
echo 'export PATH="$HOME/rCore-dev/riscv-toolchain/bin:$PATH"' >> ~/.bashrc

# 立即生效
source ~/.bashrc

# 验证安装
riscv64-unknown-elf-gcc --version
riscv64-unknown-elf-objdump --version
riscv64-unknown-elf-gdb --version
步骤 3: 创建安装脚本
bash
# 创建工具链安装脚本
cat > ~/rCore-dev/bin/install-custom-toolchain.sh << 'EOF'
#!/bin/bash
echo "Installing custom RISC-V toolchain..."

# 工具链文件名
TOOLCHAIN_FILE="riscv64-unknown-elf-gcc-8.3.0-2020.04.1-x86_64-linux-ubuntu14.tar"
DOWNLOAD_PATH="~/rCore-dev/downloads"

# 检查文件是否存在
if [ ! -f "$DOWNLOAD_PATH/$TOOLCHAIN_FILE" ]; then
    echo "Error: Toolchain file not found at $DOWNLOAD_PATH/$TOOLCHAIN_FILE"
    echo "Please download it first and place it in $DOWNLOAD_PATH/"
    exit 1
fi

echo "Found toolchain file, extracting..."

# 清理并创建目录
rm -rf ~/rCore-dev/riscv-toolchain
mkdir -p ~/rCore-dev/riscv-toolchain

# 解压
tar -xvf "$DOWNLOAD_PATH/$TOOLCHAIN_FILE" -C ~/rCore-dev/riscv-toolchain --strip-components=1

# 验证解压
if [ -f ~/rCore-dev/riscv-toolchain/bin/riscv64-unknown-elf-gcc ]; then
    echo "✓ Toolchain extracted successfully"
else
    echo "✗ Extraction failed or unexpected structure"
    exit 1
fi

# 添加到 PATH
echo 'export PATH="$HOME/rCore-dev/riscv-toolchain/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

echo "Custom toolchain installation complete!"
echo "Location: ~/rCore-dev/riscv-toolchain/"
echo "Testing..."
riscv64-unknown-elf-gcc --version
EOF

chmod +x ~/rCore-dev/bin/install-custom-toolchain.sh