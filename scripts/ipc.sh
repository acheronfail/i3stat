#!/usr/bin/env bash

echo '{"type":"'$1'"}' | socat - UNIX-CONNECT:$I3SOCK.staturs
