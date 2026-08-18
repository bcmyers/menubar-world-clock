#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
app_dir="$project_dir/target/release/World Clock.app"
contents_dir="$app_dir/Contents"

cd "$project_dir"
cargo build --release

mkdir -p "$contents_dir/MacOS"
cp "$project_dir/target/release/world-clock" "$contents_dir/MacOS/world-clock"
cp "$project_dir/Info.plist" "$contents_dir/Info.plist"
codesign --force --sign - "$app_dir"

printf '%s\n' "Built $app_dir"
