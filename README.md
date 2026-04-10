<p align="center">
<img src="media/github-header-image.png" width="833"/>
</p>

<hr/>

# wooshh

**wooshh** is a replacement for `time` written in [Rust](https://www.rust-lang.org/).

[![Release](https://github.com/mehuaniket/wooshh/actions/workflows/release.yml/badge.svg)](https://github.com/mehuaniket/wooshh/actions/workflows/release.yml)
[![codecov](https://codecov.io/gh/mehuaniket/woosh/branch/main/graph/badge.svg)](https://codecov.io/gh/mehuaniket/woosh)
[![Changelog](https://img.shields.io/badge/changelog-v0.4.0-green.svg)](https://github.com/mehuaniket/woosh/blob/main/CHANGELOG.md)
[![homebrew](https://img.shields.io/homebrew/v/wooshh.svg)](https://formulae.brew.sh/formula/wooshh)
<!-- [![Crates.io](https://img.shields.io/crates/v/wooshh.svg)](https://crates.io/crates/wooshh) -->
<!-- [![wooshh](https://snapcraft.io/wooshh/badge.svg)](https://snapcraft.io/wooshh) -->

## Documentation quick links

* [Features](#features)
* [Platform](#platform)
* [Installation](#installation)
* [Usage](#usage)
* [Configuration](#configuration)

## Features 

* Time command execution like `time`, with success/failure sound cues
* Smart notifications: only on failure or only above a duration threshold
* Desktop notifications (macOS/Linux) with command, duration, and exit code
* History tracking for run analytics
* `stats`, `slowest`, and `regressions` insights from local run history
* `compare` mode for quick performance checks (median + stddev)
* Optional command tags for grouping and tracking
* `--quiet` and `--json` output modes
* Lightweight failure hints for common toolchains (`cargo`, `npm`, etc.)

## Platform 


* Linux is supported.
* macOS is experimentally supported.

## Installation

- using script:
```
curl -LSfs https://japaric.github.io/trust/install.sh | sh -s -- --git mehuaniket/wooshh --tag v0.2.0
```

- For mac os with brew :

install
```
brew tap mehuaniket/tools https://github.com/mehuaniket/tools.git
brew install mehuaniket/tools/wooshh
```

```
brew remove wooshh
brew untap mehuaniket/toools
```
## Usage 

Run any command:

```
wooshh sleep 10
```

### Notify controls

Only notify when command is slow:

```
wooshh --notify-if-over 30s cargo test
```

Only notify on failure:

```
wooshh --on-failure-only cargo test
```

### Output modes

Quiet mode (no timing lines, still runs notifications):

```
wooshh --quiet npm run build
```

JSON output:

```
wooshh --json cargo check
```

Write output to file:

```
wooshh -o out.txt cargo test
wooshh -a -o out.txt cargo test
```

### Tags and history

Tag a run:

```
wooshh --tag build cargo build
```

Skip history write:

```
wooshh --no-history cargo test
```

History-backed insights:

```
wooshh stats
wooshh slowest -n 10
wooshh regressions -n 10
```

### Compare mode

Run a command multiple times and report median/stddev:

```
wooshh compare -n 5 cargo check
```

### Notes

* History file is stored at `~/.config/wooshh/history.toml`.
* `user/sys` fields currently stay `0.00`; `real` is measured accurately.
* On macOS, `wooshh` prefers a native helper app for notifications when available.
* Shell helper example:

```
alias w='wooshh --notify-if-over 20s'
w cargo test
```

## macOS Native Notification Setup

For the most reliable macOS notifications, build and use the bundled native helper:

```
cd notifier-macos
chmod +x build-notifier.sh
./build-notifier.sh
```

Run once to allow macOS to register the app identity:

```
open WooshhNotifier.app
```

Then test with `wooshh`:

```
cargo run -- --notify-if-over 0s sleep 1
```

Optional: use a custom location via env var:

```
WOOSHH_NOTIFIER_PATH="/path/to/WooshhNotifier.app/Contents/MacOS/wooshh-notifier" wooshh --notify-if-over 0s sleep 1
```
## License

This program is licensed under the MIT license.
