#!/usr/bin/env bash
set -euo pipefail

crate="${1:-}"
version="${2:-}"
max_attempts="${CRATES_IO_MAX_ATTEMPTS:-40}"
interval_seconds="${CRATES_IO_POLL_INTERVAL_SECONDS:-15}"
query_dir="${CRATES_IO_QUERY_DIR:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}}"

if [[ ! "$crate" =~ ^[A-Za-z0-9_-]+$ ]]; then
	echo "invalid crate name: $crate" >&2
	exit 2
fi
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
	echo "invalid crate version: $version" >&2
	exit 2
fi
if [[ ! "$max_attempts" =~ ^[1-9][0-9]*$ ]]; then
	echo "invalid CRATES_IO_MAX_ATTEMPTS: $max_attempts" >&2
	exit 2
fi
if [[ ! "$interval_seconds" =~ ^[0-9]+$ ]]; then
	echo "invalid CRATES_IO_POLL_INTERVAL_SECONDS: $interval_seconds" >&2
	exit 2
fi
if [[ ! -d "$query_dir" ]]; then
	echo "crates.io query directory does not exist: $query_dir" >&2
	exit 2
fi

for ((attempt = 1; attempt <= max_attempts; attempt += 1)); do
	if (cd "$query_dir" && cargo info --registry crates-io "$crate@$version" >/dev/null 2>&1); then
		echo "$crate $version is visible in the crates.io index"
		exit 0
	fi
	if ((attempt < max_attempts)); then
		echo "waiting for $crate $version in crates.io index ($attempt/$max_attempts)"
		sleep "$interval_seconds"
	fi
done

echo "$crate $version did not reach the crates.io index after $max_attempts attempts" >&2
exit 1
