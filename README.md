# `mc-tools` (Name sudgestions welcome)
A collection of useful tools related to minecraft networking written in rust

## Crates
- `bots`: Stress testing tool for minecraft server implementations
- `mc_io`: Packet reading and writing infrastructure
- `proto`: Minecraft packet definetions for 1.19.2

# `bots`
**DISCLAIMER**: Usage of this stress testing tool for purposes than testing
your own infrastructure can be seen as **illeagal** in many countries

A tool to stress test minecraft server by connecting artificial players

## Usage

```sh
# Clone repo
git clone https://github.com/Eoghanmc22/mc-tools.git

# Compile
cargo build --release --bin bots
# Binary will be located at ./target/release/bots

# Alternativly, use cargo install
cargo install --path .
# Binary avaible as `bots` command

# Simple usage
./path/to/bots [options] <server_ip>:[port] <count>
./path/to/bots localhost 1000

# Read help to see a list of options
./path/to/bots --help
```
