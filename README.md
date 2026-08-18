# World Clock

A small, local-only macOS menu bar world clock written in Rust. The UI uses
native Objective-C AppKit objects through the `objc2` bindings: `NSStatusItem`,
`NSMenu`, custom `NSView` rows, and `NSTextField` labels.

Each clock row has a dedicated city field and time field. The time fields are
left-aligned inside a fixed-width column placed against the menu's right inset.
Both columns use AppKit's monospaced system font. Times use the format
`Tue 23:59:59`, with no time-zone abbreviation.

## Run during development

```sh
cargo run
```

Click the globe item in the macOS menu bar. Choose **Quit World Clock** when done.

## Build a standalone app bundle

```sh
./scripts/build-app.sh
open "target/release/World Clock.app"
```

The build stays entirely local. This project does not initialize a Git
repository or create a remote repository.

## Keep it running with launchd

Install the signed app and load its per-user LaunchAgent:

```sh
./scripts/install-launch-agent.sh
```

The LaunchAgent starts the app at login and restarts it whenever it exits.
Because this is an always-running service, choosing **Quit World Clock** causes
launchd to start it again after a short delay.

Runtime files follow Linux/XDG conventions:

1. App: `~/.local/lib/menubar-world-clock/World Clock.app`
2. Configuration: `${XDG_CONFIG_HOME:-~/.config}/menubar-world-clock/config.toml`
3. LaunchAgent source: `${XDG_CONFIG_HOME:-~/.config}/menubar-world-clock/launchd.plist`
4. Logs: `${XDG_STATE_HOME:-~/.local/state}/menubar-world-clock/launchd.log`

macOS requires a LaunchAgent directory entry at
`~/Library/LaunchAgents/local.world-clock.plist` for automatic login startup.
The installer makes that entry a hard link to the canonical plist under
`XDG_CONFIG_HOME`; no application data or logs are stored under `~/Library`.

To unload and remove the LaunchAgent while leaving the installed app in place:

```sh
./scripts/uninstall-launch-agent.sh
```

## Configure clocks

On first launch, World Clock creates:

```text
~/.config/menubar-world-clock/config.toml
```

The home directory is located with the third-party Rust `dirs` crate. The
initial file contains San Francisco, New York, UTC, and Paris. Edit it using
IANA time-zone names:

```toml
[[clocks]]
city = "Tokyo"
timezone = "Asia/Tokyo"

[[clocks]]
city = "Sydney"
timezone = "Australia/Sydney"
```

Choose **Open Configuration…** from the menu to open the file, then choose
**Reload Configuration** after saving. Invalid or empty configurations are
rejected without replacing the clocks currently shown; details are written to
standard error when running from a terminal.
