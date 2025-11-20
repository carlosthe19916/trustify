#!/bin/bash

# custom initialization goes here - runs outside of the dev container
# just before the container is launched but after the container is created

echo "devcontainerID ${1}"

KC_HOSTNAME="${CODESPACE_NAME:+${CODESPACE_NAME}-9090.app.github.dev}"
UI_ISSUER_URL="${CODESPACE_NAME:+${CODESPACE_NAME}-9090.app.github.dev/realms/trustify}"

{
  echo "KC_HOSTNAME=\"$KC_HOSTNAME\""
  echo "UI_ISSUER_URL=\"$UI_ISSUER_URL\""
} > .env