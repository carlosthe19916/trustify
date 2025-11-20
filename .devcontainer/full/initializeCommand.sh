#!/bin/bash

# custom initialization goes here - runs outside of the dev container
# just before the container is launched but after the container is created

echo "devcontainerID ${1}"

if [ "${CODESPACES}" = "true" ]; then
    > .devcontainer/full/.env

    KC_HOSTNAME="https://${CODESPACE_NAME}-9090.${GITHUB_CODESPACES_PORT_FORWARDING_DOMAIN}"
    UI_ISSUER_URL="https://${CODESPACE_NAME}-9090.${GITHUB_CODESPACES_PORT_FORWARDING_DOMAIN}/realms/trustify"

    echo "KC_HOSTNAME=$KC_HOSTNAME" >> .devcontainer/full/.env
    echo "UI_ISSUER_URL=$UI_ISSUER_URL" >> .devcontainer/full/.env
fi
