#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
label="local.world-clock"
domain="gui/$(id -u)"
xdg_config_home=${XDG_CONFIG_HOME:-"$HOME/.config"}
xdg_state_home=${XDG_STATE_HOME:-"$HOME/.local/state"}
install_dir="$HOME/.local/lib/menubar-world-clock"
installed_app="$install_dir/World Clock.app"
config_dir="$xdg_config_home/menubar-world-clock"
canonical_agent="$config_dir/launchd.plist"
state_dir="$xdg_state_home/menubar-world-clock"
log_path="$state_dir/launchd.log"
launch_agents_dir="$HOME/Library/LaunchAgents"
agent_shim="$launch_agents_dir/$label.plist"
template="$project_dir/launchd/$label.plist"
legacy_config="$HOME/.config/world-clock/config.toml"
legacy_app="$HOME/Applications/World Clock.app"
legacy_log="$HOME/Library/Logs/World Clock.launchd.log"

"$project_dir/scripts/build-app.sh"

if launchctl print "$domain/$label" >/dev/null 2>&1; then
    launchctl bootout "$domain/$label"
fi

# Stop an already-running development or installed copy before replacing it.
pkill -f '/World Clock.app/Contents/MacOS/world-clock' 2>/dev/null || true

mkdir -p "$install_dir" "$config_dir" "$state_dir" "$launch_agents_dir"

if [ -f "$legacy_config" ] && [ ! -e "$config_dir/config.toml" ]; then
    mv "$legacy_config" "$config_dir/config.toml"
fi

ditto "$project_dir/target/release/World Clock.app" "$installed_app"

cp "$template" "$canonical_agent"
plutil -insert ProgramArguments.3 -string "$installed_app" "$canonical_agent"
plutil -replace StandardOutPath -string "$log_path" "$canonical_agent"
plutil -replace StandardErrorPath -string "$log_path" "$canonical_agent"
plutil -lint "$canonical_agent"

# launchd only discovers login agents in ~/Library/LaunchAgents. Keep the
# canonical plist under XDG_CONFIG_HOME and expose the required directory entry
# as a hard link, rather than maintaining a second configuration file there.
rm -f "$agent_shim"
ln "$canonical_agent" "$agent_shim"

launchctl bootstrap "$domain" "$agent_shim"
launchctl enable "$domain/$label"
launchctl kickstart -k "$domain/$label"

rm -rf "$legacy_app"
rm -f "$legacy_log"

printf '%s\n' "Installed $installed_app"
printf '%s\n' "Configuration directory: $config_dir"
printf '%s\n' "State directory: $state_dir"
printf '%s\n' "Loaded $canonical_agent via required launchd shim $agent_shim"
