#!/bin/bash
case "$1" in -V|-v|-?) exec /usr/bin/dot "$@";; esac
f=${DOTDIR:-/tmp/dots}/$(date +%s%N)-$$
printf '%s\n' "$@" > $f.args
tee $f.dot | /usr/bin/dot "$@"
