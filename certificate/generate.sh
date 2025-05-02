#!/usr/bin/env bash

if [ $# -lt 1 ] ; then
    echo "usage: $0 <privkey-password> " >&2
    exit 1
fi

rm -fv *.pem *.csr 

PRIV_KEY=priv-key.pem
PUB_KEY=pub-key.pem

password="pass:$1"

openssl genpkey -out $PRIV_KEY -outpubkey $PUB_KEY \
    -algorithm rsa -pkeyopt rsa_keygen_bits:2048   \
    -aes-256-cbc -pass "$password"

chmod 600 *.pem
openssl req -new -key $PRIV_KEY -passin "$password" -config cert.conf -out certificate.csr
