#!/bin/bash

source .env

if [[ -z "${@}" ]]; then
  echo "login to zcore shell: (default user/pass: zcore/zcore)"
else
  echo "executing shell command in zcore: $@"
fi
echo ""
ssh -X -p 9000 ${MY_NAME}@127.0.0.1 $@

