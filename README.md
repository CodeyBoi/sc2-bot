# Prerequisites
To run, you must first install [Rust](https://www.rust-lang.org/tools/install), and a version of StarCraft II.

## StarCraft II

### Windows or MacOS
1. Install SC2 via [Battle.net](https://download.battle.net/en-us/desktop).
2. Download a [Map Pack](https://github.com/Blizzard/s2client-proto?tab=readme-ov-file#map-packs) place it into your StarCraft II-folder in the directory `Maps` (create it if it doesn't exist).

### Linux
The Linux installation will be headless (I haven't tested the headfull version yet).

#### Headless
1. Download the most recent [SC2 Linux Package](https://github.com/Blizzard/s2client-proto#linux-packages).
2. Unzip to `~/StarCraftII` (the password for the archive is available above the Linux Packages link).
3. Move your `.SC2Map`-files out of their `LadderXXXXSeasonX` directory to the `Maps` directory.

# Running the bot
To run the bot execute the following commands;
1. `export SC2PATH=/path/to/StarCraftII`
2. `cargo run`

*Note:* On Windows I could only get this running correctly via Git Bash, and then by running `export SC2PATH='/c/Program Files (x86)/StarCraft II'; cargo run`.

# Reading docs
Documentation can be compiled and opened in a web browser by running `cargo doc --open`.
