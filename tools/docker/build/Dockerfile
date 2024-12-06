FROM ubuntu:20.04

ENV DEBIAN_FRONTEND=noninteractive
ENV INSTALL_PREFIX=/opt/zcore

ARG username
ARG password
ARG timezone

RUN apt-get update && apt-get install -y openssh-server
RUN mkdir /var/run/sshd

RUN adduser --disabled-login --gecos '' $username
RUN echo $username:$password | chpasswd
RUN adduser $username sudo
RUN adduser $username plugdev

RUN sed -ri 's/^#?#X11UseLocalhost\s+.*/X11UseLocalhost no/' /etc/ssh/sshd_config
RUN sed -ri 's/UsePAM yes/#UsePAM yes/g' /etc/ssh/sshd_config

RUN echo $timezone > /etc/timezone

RUN apt-get install -y software-properties-common

RUN dpkg --add-architecture i386
RUN add-apt-repository universe
RUN apt-get update --fix-missing

RUN apt-get install -y \
        build-essential \
        pkg-config \
        python3 \
        python3-pip \
        meson \
        libglib2.0-dev \
        libpixman-1-dev \
        xz-utils \
        wget \
        curl \
        locales \
        sudo \
        bash-completion \
        vim

RUN apt-get clean && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# Install git lfs
RUN curl -s https://packagecloud.io/install/repositories/github/git-lfs/script.deb.sh | bash \
    && apt-get install -y git-lfs \
    && git lfs install

# Install QEMU
RUN mkdir -p ${INSTALL_PREFIX} \
    && wget https://download.qemu.org/qemu-7.0.0.tar.xz \
    && tar -xvJf qemu-7.0.0.tar.xz -C ${INSTALL_PREFIX} \
    && rm -rf qemu-7.0.0.tar.xz \
    && ln -s ${INSTALL_PREFIX}/qemu-7.0.0 ${INSTALL_PREFIX}/qemu \
    && cd ${INSTALL_PREFIX}/qemu \
    && ./configure --target-list=x86_64-softmmu,x86_64-linux-user,riscv64-softmmu,riscv64-linux-user,aarch64-softmmu,aarch64-linux-user \
    && make -j `nproc` \
    && make install \
    && rm -rf ${INSTALL_PREFIX}/qemu/*

# Install rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

RUN locale-gen en_US.UTF-8

RUN sed -ri 's/^%sudo\s+ALL=\(ALL:ALL\)\s+ALL/%sudo ALL=(ALL:ALL) NOPASSWD:ALL/' /etc/sudoers

ENV PATH="$HOME/.cargo/bin:$PATH"
ENV WORK_SPACE_PATH=${INSTALL_PREFIX}/zcore

WORKDIR ${WORK_SPACE_PATH}
COPY . .

CMD    ["/usr/sbin/sshd", "-D"]

