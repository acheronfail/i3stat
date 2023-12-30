FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main

ENV DEBIAN_FRONTEND=noninteractive TZ=Europe/London
RUN dpkg --add-architecture arm64 && apt-get update

RUN apt-get install -y \
  build-essential \
  clang \
  dbus \
  dunst \
  i3-wm \
  imagemagick \
  libfaketime \
  libiw-dev \
  libx11-dev:arm64 \
  scrot \
  xserver-xephyr \
  xvfb

# NOTE: we build and install libpulse manually ourselves, since the version in
# cross' current image is too low (v13). We need at least v14.
RUN apt-get install -y \
  autopoint \
  bash-completion \
  check:arm64 \
  curl \
  dbus-x11 \
  dpkg-dev \
  g++ \
  gcc \
  gettext \
  git-core \
  libasound2-dev \
  libasyncns-dev \
  libavahi-client-dev \
  libbluetooth-dev \
  libcap-dev:arm64 \
  libdbus-1-dev:arm64 \
  libfftw3-dev \
  libglib2.0-dev \
  libgtk-3-dev \
  libice-dev \
  libjack-dev \
  liblircclient-dev \
  libltdl-dev:arm64 \
  liborc-0.4-dev \
  libsbc-dev:arm64 \
  libsndfile1-dev:arm64 \
  libsoxr-dev \
  libspeexdsp-dev \
  libssl-dev \
  libsystemd-dev \
  libtdb-dev:arm64 \
  libudev-dev \
  libwebrtc-audio-processing-dev \
  libwrap0-dev \
  libx11-xcb-dev \
  libxcb1-dev \
  libxml-parser-perl \
  libxml2-utils \
  libxtst-dev \
  m4 \
  meson \
  ninja-build \
  pkg-config:arm64 \
  python3-setuptools \
  systemd \
  wget
RUN rm -rf /opt/pulseaudio && git clone \
  --depth 1 \
  --branch stable-14.x \
  https://gitlab.freedesktop.org/pulseaudio/pulseaudio.git \
  /opt/pulseaudio
COPY ./cross/aarch64-meson.txt /opt/pulseaudio/aarch64-meson.txt
RUN cd /opt/pulseaudio && meson --cross-file aarch64-meson.txt build && cd build && ninja && ninja install
