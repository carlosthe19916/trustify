#!/bin/bash

# Custom initialization goes here if needed.
# Runs inside the dev container after the container is created

################################################################################
# When using docker we will not be root inside the container
# the following steps are then required
################################################################################

echo "alias start:dev='cargo run --bin trustd db migrate && cargo run --bin trustd api'" >> ~/.bashrc
echo "alias psql:postgres='env PGPASSWORD=trustify psql -U postgres -d postgres -h trustify-db -p 5432'" >> ~/.bashrc
