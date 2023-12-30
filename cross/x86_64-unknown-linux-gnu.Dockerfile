FROM ghcr.io/cross-rs/x86_64-unknown-linux-gnu:main

ENV DEBIAN_FRONTEND=noninteractive TZ=Europe/London

RUN apt-get install -y \
  build-essential \
  clang \
  dbus \
  dunst \
  i3-wm \
  imagemagick \
  libfaketime \
  libiw-dev \
  libx11-dev \
  scrot \
  xserver-xephyr \
  xvfb

# NOTE: we build and install libpulse manually ourselves, since the version in
# cross' current image is too low (v13). We need at least v14.
RUN apt-get install -y \
  autopoint \
  bash-completion \
  check \
  curl \
  dbus-x11 \
  g++ \
  gcc \
  gettext \
  git-core \
  libasound2-dev \
  libasyncns-dev \
  libavahi-client-dev \
  libbluetooth-dev \
  libcap-dev \
  libfftw3-dev \
  libglib2.0-dev \
  libgtk-3-dev \
  libice-dev \
  libjack-dev \
  liblircclient-dev \
  libltdl-dev \
  liborc-0.4-dev \
  libsbc-dev \
  libsndfile1-dev \
  libsoxr-dev \
  libspeexdsp-dev \
  libssl-dev \
  libsystemd-dev \
  libtdb-dev \
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
  pkg-config \
  python3-setuptools \
  systemd \
  wget
RUN rm -rf /opt/pulseaudio && git clone \
  --depth 1 \
  --branch stable-14.x \
  https://gitlab.freedesktop.org/pulseaudio/pulseaudio.git \
  /opt/pulseaudio
RUN cd /opt/pulseaudio && meson build && cd build && ninja && ninja install
