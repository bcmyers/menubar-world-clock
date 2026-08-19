# World Clock

A Rust/AppKit menu bar clock for San Francisco, New York, UTC, Paris, or any
configured IANA time zone. It shows `Tue 23:59:59` using aligned monospaced
columns.

## Run

```sh
cargo run
```

Build the app bundle:

```sh
./scripts/build-app.sh
```

## Start at login

Install the app and its always-running LaunchAgent:

```sh
./scripts/install-launch-agent.sh
```

Remove the LaunchAgent while keeping the installed app:

```sh
./scripts/uninstall-launch-agent.sh
```

## Configure

Edit `~/.config/menubar-world-clock/config.toml`:

```toml
[[clocks]]
city = "Tokyo"
timezone = "Asia/Tokyo"
```

Then choose **Reload Configuration** from the menu.

## Runtime paths

1. App: `~/.local/lib/menubar-world-clock/World Clock.app`
2. Config: `~/.config/menubar-world-clock/config.toml`
3. Logs: `~/.local/state/menubar-world-clock/launchd.log`

The required `~/Library/LaunchAgents/local.world-clock.plist` entry is a hard
link to the canonical plist under `~/.config/menubar-world-clock/`.

## License

Licensed under either Apache 2.0 or MIT, at your option.
