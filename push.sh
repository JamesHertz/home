#!/usr/bin/env bash
set -e

make
ssh home "sudo systemctl stop server.service"
rsync bin/server home_server:~/server
rsync -av ./frontend/ home_server:~/.static-files
ssh home "sudo systemctl start server.service"
