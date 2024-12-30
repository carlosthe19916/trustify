apt-get update

# Install OS Dependencies
apt-get -y install pkg-config

## Git autocomplete
echo "source /usr/share/bash-completion/completions/git" >> ~/.bashrc

## Initialize DB
cargo run --bin trustd db migrate