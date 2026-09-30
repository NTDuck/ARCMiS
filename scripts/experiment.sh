#!/usr/bin/env bash
# Run one experiment: one harness candidate, one problem set, one
# immutable directory under .artifacts/experiments/.
#
# Usage: experiment.sh <config.yml> <candidate-id> [parents] [hypothesis]
#   parents:    comma-separated experiment ids, empty for a seed
#   hypothesis: free text recorded in the manifest
#
# The script renders the config with the experiment workspace, runs the
# harness with the experiment dir as the second argument, and records
# stdout, stderr, and the exit code. Nothing aggregates here. The
# frontier script reads the whole tree.
set -u
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO" || exit 1

CONFIG_SRC="${1:?usage: experiment.sh <config.yml> <candidate-id> [parents] [hypothesis]}"
CANDIDATE="${2:?usage: experiment.sh <config.yml> <candidate-id> [parents] [hypothesis]}"
PARENTS="${3:-}"
HYPOTHESIS="${4:-}"

EXP_ROOT="$REPO/.artifacts/experiments"
OUT="$EXP_ROOT/$CANDIDATE"
if [ -e "$OUT" ]; then
  echo "experiment dir already exists: $OUT" >&2
  exit 1
fi
mkdir -p "$OUT"

# Render the config: output.dir points at the experiment dir itself. The
# harness derives workspace/ (source, target, analysis), run/ (blackboard),
# and traces/ from it, so every artifact of one candidate sits in one place.
cfg="$OUT/config.yml"
CONFIG_PATH="$CONFIG_SRC"
case "$CONFIG_SRC" in
  /*) ;;
  *) CONFIG_PATH="$REPO/$CONFIG_SRC" ;;
esac
sed "s#^[[:space:]]*dir:[[:space:]].*#  dir: $OUT#" "$CONFIG_PATH" > "$cfg"

# Lineage: the manifest written by the harness carries parents and
# hypothesis as plain text. Patch them in after the run.
# Run timeout is env-overridable. The 7200 default fits the runs that
# historically died at 3600 mid-Hardening; prior 3600-second runs stay
# comparable because each manifest records the wrapper rc.
TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-7200}"
# v3 engine policy (binding): ninfer (OpenAI-compatible :8081) through
# the harness netmind provider path is the default engine. Ollama is
# not to be used. Override only with an explicit author directive.
NETMIND_BASE_URL="${NETMIND_BASE_URL:-http://localhost:8081/v1}" \
NETMIND_API_KEY="${NETMIND_API_KEY:-x}" \
  timeout "$TIMEOUT_SECONDS" cargo run --quiet -p harness --manifest-path "$REPO/Cargo.toml" -- \
    --config "$cfg" --experiment "$CANDIDATE" \
  > "$OUT/stdout.log" 2> "$OUT/stderr.log"
rc=$?

python3 "$SCRIPT_DIR/lineage.py" "$OUT/manifest.json" "$PARENTS" "$HYPOTHESIS"

echo "{\"candidate\":\"$CANDIDATE\",\"config\":\"$CONFIG_SRC\",\"rc\":$rc}" >> "$EXP_ROOT/experiments.log"
echo "experiment $CANDIDATE rc=$rc dir=$OUT"
exit "$rc"
