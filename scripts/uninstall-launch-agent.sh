#!/bin/sh
set -eu

label="local.world-clock"
domain="gui/$(id -u)"
xdg_config_home=${XDG_CONFIG_HOME:-"$HOME/.config"}
canonical_agent="$xdg_config_home/menubar-world-clock/launchd.plist"
agent_shim="$HOME/Library/LaunchAgents/$label.plist"

if launchctl print "$domain/$label" >/dev/null 2>&1; then
    launchctl bootout "$domain/$label"
fi

pkill -f '/World Clock.app/Contents/MacOS/world-clock' 2>/dev/null || true
rm -f "$agent_shim" "$canonical_agent"

printf '%s\n' "Unloaded and removed the $label LaunchAgent"
printf '%s\n' "The app remains installed at $HOME/.local/lib/menubar-world-clock/World Clock.app"
