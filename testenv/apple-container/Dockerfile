FROM ubuntu:24.04

LABEL org.opencontainers.image.description="Linux smoke host for rig (sshd + Rust). Apple container / local only."
LABEL org.opencontainers.image.licenses="MIT"

ENV DEBIAN_FRONTEND=noninteractive
ENV SSH_USER=dev
ENV RUSTUP_HOME=/opt/rustup
ENV CARGO_HOME=/opt/cargo
ENV PATH=/opt/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    git \
    openssh-client \
    openssh-server \
    pkg-config \
    python3 \
    rsync \
    sudo \
    vim-tiny \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash "${SSH_USER}" \
    && echo "${SSH_USER} ALL=(ALL) NOPASSWD:ALL" > "/etc/sudoers.d/${SSH_USER}" \
    && chmod 440 "/etc/sudoers.d/${SSH_USER}"

# Host-arch Rust toolchain (linux/arm64 under Apple container on M-series)
RUN curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal \
    && chmod -R a+rX /opt/rustup /opt/cargo \
    && printf '%s\n' \
         'export RUSTUP_HOME=/opt/rustup' \
         'export CARGO_HOME=/opt/cargo' \
         'export PATH="/opt/cargo/bin:$PATH"' \
       > /etc/profile.d/rust.sh \
    && printf '%s\n' \
         'RUSTUP_HOME=/opt/rustup' \
         'CARGO_HOME=/opt/cargo' \
         'PATH=/opt/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin' \
       >> /etc/environment

RUN mkdir -p /run/sshd /mounted-keys \
    && mkdir -p "/home/${SSH_USER}/.ssh" \
    && chmod 700 "/home/${SSH_USER}/.ssh" \
    && touch "/home/${SSH_USER}/.ssh/authorized_keys" \
    && chmod 600 "/home/${SSH_USER}/.ssh/authorized_keys" \
    && chown -R "${SSH_USER}:${SSH_USER}" "/home/${SSH_USER}/.ssh" \
    && sed -i 's/#PubkeyAuthentication.*/PubkeyAuthentication yes/' /etc/ssh/sshd_config \
    && sed -i 's/#PermitRootLogin.*/PermitRootLogin no/' /etc/ssh/sshd_config \
    && ssh-keygen -A

COPY start.sh /start.sh
RUN chmod +x /start.sh

EXPOSE 22
CMD ["/bin/bash", "/start.sh"]
