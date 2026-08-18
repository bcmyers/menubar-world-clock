#!/bin/sh
set -eu

label="local.world-clock"
domain="gui/$(id -u)"
agent_path="$HOME/Library/LaunchAgents/$label.plist"

if launchctl print "$domain/$label" >/dev/null 2>&1; then
    launchctl bootout "$domain/$label"
fi

pkill -f '/World Clock.app/Contents/MacOS/world-clock' 2>/dev/null || true
rm -f "$agent_path"

printf '%s\n' "Unloaded and removed $agent_path"
printf '%s\n' "The app remains installed at $HOME/Applications/World Clock.app"

