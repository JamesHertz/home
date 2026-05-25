#!/usr/bin/env bash

if [ $# -lt 1 ] ; then
	echo "usage: $0 <domain>"
	exit 1
fi

domain=$1
echo "requesting certificate for '$domain'"

echo -n "press enter to continue ... "
read

mkdir -pv certs

docker run --rm -it -v "$(pwd)/certs":/acme.sh -p 80:80 \
	neilpang/acme.sh --issue -d "$domain" --standalone  \
	--server letsencrypt
