#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
label="local.world-clock"
domain="gui/$(id -u)"
applications_dir="$HOME/Applications"
installed_app="$applications_dir/World Clock.app"
launch_agents_dir="$HOME/Library/LaunchAgents"
agent_path="$launch_agents_dir/$label.plist"
logs_dir="$HOME/Library/Logs"
log_path="$logs_dir/World Clock.launchd.log"
template="$project_dir/launchd/$label.plist"

"$project_dir/scripts/build-app.sh"

if launchctl print "$domain/$label" >/dev/null 2>&1; then
    launchctl bootout "$domain/$label"
fi

# Stop an already-running development or installed copy before replacing it.
pkill -f '/World Clock.app/Contents/MacOS/world-clock' 2>/dev/null || true

mkdir -p "$applications_dir" "$launch_agents_dir" "$logs_dir"
ditto "$project_dir/target/release/World Clock.app" "$installed_app"

cp "$template" "$agent_path"
plutil -insert ProgramArguments.3 -string "$installed_app" "$agent_path"
plutil -replace StandardOutPath -string "$log_path" "$agent_path"
plutil -replace StandardErrorPath -string "$log_path" "$agent_path"
plutil -lint "$agent_path"

launchctl bootstrap "$domain" "$agent_path"
launchctl enable "$domain/$label"
launchctl kickstart -k "$domain/$label"

printf '%s\n' "Installed $installed_app"
printf '%s\n' "Loaded $agent_path"
