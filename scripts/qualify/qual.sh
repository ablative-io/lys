#!/bin/bash
# BOX 20 qualification, DIRECTORY-064 R6 as amendments 37 and 38 rule it. Archie, 7 Oct 2026.
#
# Run from Waffles' hands, in his window, on Tom's Mac. Nothing here compiles, tests or builds; the
# qualify_adapter example is built on Dean at the written candidate and copied here like the BOX 17
# artifacts. Every verdict is read from parsed output, never from an exit code. Each stage ends with
# one line that starts "END ", and that line is the result.
#
# The window's order (amendment 37):
#   qual.sh versions stage-a  --out DIR    record claude and codex versions before Stage A
#   qual.sh a --out DIR --example PATH     Stage A: protocol qualification, writes nothing to Lys
#   (pin commit, citing DIR/stage-a/pins.txt; then the one battery)
#   qual.sh versions battery  --out DIR    at the battery's start
#   (the one install)
#   qual.sh b --out DIR --helper PATH --machine ID --wait SECONDS [--base URL]
#                                          Stage B against the install; teardown always runs
#
# WHAT STAGE A WRITES: only DIR, and each harness's own session data in its own data directory,
# as any run of that harness on this Mac does. It reads no harness data directory and never
# touches ~/.codex. It never reads a credential or an environment value.
#
# WHAT STAGE B WRITES TO LIVE LYS. All of it is done as the administrator Waffles' helper signs in,
# and all of it concerns qual-box20 only. Each write is appended to DIR/stage-b/writes.tsv before
# it is sent and gets its answer status after:
#   1. Add an agent screen: registers qual-box20, activates it, saves and reviews its profile, and
#      allows it on --machine (POST /agents, /identities/{id}/transitions activate,
#      /agents/{id}/provisioning and its review, /network/machines/{m}/agents allow:true)
#   2. Usage screen: one context threshold (PUT /budgets/agent/{id}) and one goal with its
#      reminder (POST /agents/{id}/goals)
#   3. for each harness: a profile version selecting that harness with
#      session.requires_controls=true, its review, and one managed start (POST
#      /agents/{id}/start-command)
#   4. for each harness: the turn act, the threshold crossing it causes and its compaction, then
#      one stop (BIND below)
#   5. teardown: the goal switched off (POST /goals/{g}/active false), the machine allowance
#      removed (allow:false), and qual-box20 retired (POST /identities/{id}/transitions retire)
# Lys deletes nothing. Every record above stays, and teardown lists them in DIR/stage-b/records.txt.
#
# The one stop on this Mac (Waffles 438713ed, until Tom rules otherwise): available disk below
# 20 GB halts the leg, naming the reading, so the disk can never fill. Read before every leg. There
# is no start bar and no memory or swap bar. An unread disk is no verdict, and the leg is refused.

set -u
export LC_ALL=C
HERE=$(cd "$(dirname "$0")" && pwd)
JUDGE="/usr/bin/python3 $HERE/judge.py"
AGENT=qual-box20
SESSION=qual-box20
STAGE=${1:-}
shift || true
OUT='' EXAMPLE='' HELPER='' MACHINE='' WAIT='' BASE=http://127.0.0.1:8490 LABEL=''
if [ "$STAGE" = versions ]; then LABEL=${1:-}; shift || true; fi
while [ $# -gt 0 ]; do
  case "$1" in
    --out) OUT=$2 ;; --example) EXAMPLE=$2 ;; --helper) HELPER=$2 ;; --machine) MACHINE=$2 ;;
    --wait) WAIT=$2 ;; --base) BASE=$2 ;; *) echo "END stage=$STAGE result=REFUSED reason=unknown-argument:$1"; exit 2 ;;
  esac
  shift 2
done
[ -n "$OUT" ] || { echo "END stage=$STAGE result=REFUSED reason=no-out"; exit 2; }
mkdir -p "$OUT"
TRACE="$OUT/trace-$STAGE.log"
note() { printf '%s %s\n' "$(date '+%H:%M:%S')" "$*" | tee -a "$TRACE" >&2; }

# The machine's own harness setup: a CLAUDE_CONFIG_DIR a person did not choose for this run is
# refused, never unset or changed. Only whether it is set is read.
if [ -n "${CLAUDE_CONFIG_DIR+set}" ]; then
  echo "END stage=$STAGE result=REFUSED reason=CLAUDE_CONFIG_DIR-is-set"; exit 2
fi

guard() {
  local disk
  disk=$(df -g / | awk 'NR==2 {print $4}')
  case "$disk" in *[!0-9]*|'') note "guard unread disk=$disk"; return 1 ;; esac
  note "guard disk=${disk}G"
  [ "$disk" -ge 20 ] || { note "guard halt: available disk ${disk}G is below 20G"; return 1; }
}

versions() {
  local file="$OUT/versions-$1.raw"
  { claude --version 2>&1; codex --version 2>&1; } > "$file"
  $JUDGE versions "$file" > "$OUT/versions-$1.txt"
  note "versions $1: $(tr '\n' ' ' < "$OUT/versions-$1.txt")"
}

# ---------------------------------------------------------------------------------- versions
if [ "$STAGE" = versions ]; then
  [ -n "$LABEL" ] || { echo "END stage=versions result=REFUSED reason=no-label"; exit 2; }
  versions "$LABEL"
  unread=$(grep -c ' unread$' "$OUT/versions-$LABEL.txt")
  [ "$unread" = 0 ] && result=RECORDED || result=INCOMPLETE
  echo "END stage=versions label=$LABEL result=$result $(tr '\n' ' ' < "$OUT/versions-$LABEL.txt")"
  exit 0
fi

# ---------------------------------------------------------------------------------- Stage A
if [ "$STAGE" = a ]; then
  [ -x "$EXAMPLE" ] || { echo "END stage=a result=REFUSED reason=example-not-executable"; exit 2; }
  [ -s "$OUT/versions-stage-a.txt" ] || versions stage-a
  mkdir -p "$OUT/stage-a"
  : > "$OUT/stage-a/pins.txt"
  passed=0 failed=''
  for adapter in claude-code codex; do
    guard || { failed="$failed $adapter:guard"; continue; }
    work="$OUT/stage-a/work-$adapter"
    evidence="$OUT/stage-a/evidence-$adapter.jsonl"
    rm -rf "$work" && mkdir -p "$work"
    note "stage a $adapter: launching the example"
    "$EXAMPLE" --adapter "$adapter" --directory "$work" --evidence "$evidence" \
      > "$OUT/stage-a/example-$adapter.out" 2>&1
    note "stage a $adapter: example exit $? (not the verdict)"
    [ -s "$evidence" ] || { failed="$failed $adapter:no-evidence"; continue; }
    verdict=$($JUDGE stage-a "$evidence" "$adapter")
    note "stage a $verdict"
    expected=$(awk -v a="$adapter" '$1==a {print $2}' "$OUT/versions-stage-a.txt")
    set -- $verdict
    if [ "$1" = PASS ] && [ "$3" = "$expected" ]; then
      printf '%s %s %s %s\n' "$2" "$3" "$4" "$evidence" >> "$OUT/stage-a/pins.txt"
      passed=$((passed + 1))
    elif [ "$1" = FIXTURE ]; then
      failed="$failed $adapter:fixture-evidence-is-never-a-pin"
    elif [ "$1" = PASS ]; then
      failed="$failed $adapter:proved-$3-but-version-reads-$expected"
    else
      failed="$failed $adapter:$3:$4"
    fi
  done
  [ "$passed" = 2 ] && result=PASS || result=FAIL
  echo "END stage=a result=$result proved=$passed failed=[${failed# }] pins=$OUT/stage-a/pins.txt"
  exit 0
fi

# ---------------------------------------------------------------------------------- Stage B
[ "$STAGE" = b ] || { echo "END stage=$STAGE result=REFUSED reason=unknown-stage"; exit 2; }
for need in HELPER MACHINE WAIT; do
  eval "value=\${$need}"
  [ -n "$value" ] || { echo "END stage=b result=REFUSED reason=no-$(echo $need | tr A-Z a-z)"; exit 2; }
done
case "$WAIT" in *[!0-9]*) echo "END stage=b result=REFUSED reason=wait-not-seconds"; exit 2 ;; esac
[ -x "$HELPER" ] || { echo "END stage=b result=REFUSED reason=helper-not-executable"; exit 2; }
[ -s "$OUT/stage-a/pins.txt" ] || { echo "END stage=b result=REFUSED reason=no-stage-a-pins"; exit 2; }
B="$OUT/stage-b"
mkdir -p "$B"
: > "$B/writes.tsv"
LEGS='' FAILED='' INCOMPLETE=''
leg() { LEGS="$LEGS $1:$2"; case "$2" in FAIL*) FAILED="$FAILED $1" ;; INCOMPLETE*) INCOMPLETE="$INCOMPLETE $1" ;; esac; note "leg $1 $2"; }

# BIND: every name Stage B depends on, with where it was read. UNBOUND refuses Stage B before
# any write, by name. The R5 and R6 names are filled in when Brisket's surface and routes are
# green; until then they are UNBOUND (draft 93e19e73 has none of them).
BIND=$(cat <<'EOF'
screen.add_agent|/agents/new|main c03a38bb routes.tsx:49
screen.usage|/file/{agent}/usage|main c03a38bb routes.tsx:42-53 (usage moved into the file)
label.add.name|Name|main AddAgent.tsx
label.add.computer|Computer|main AddAgent.tsx
label.add.submit|UNBOUND|AddAgent.tsx:194 Act: 'Add agent', not the one-press 'Add ... and run it' (that starts before the process baseline)
label.add.harness|UNBOUND|R6: the profile's harness choice
label.add.requires_controls|UNBOUND|R6: session.requires_controls on the profile
label.usage.threshold|UNBOUND|R6: context limit field on Usage
label.usage.threshold_action|UNBOUND|R6: context limit action on Usage
label.usage.threshold_save|UNBOUND|R6
label.goal.words|words|main UsageGoals.tsx name=words
label.goal.deadline|deadline|main UsageGoals.tsx name=deadline
label.goal.reminder|UNBOUND|R6: the goal's reminder choice
label.goal.save|Set this goal|main UsageGoals.tsx
text.usage.held|UNBOUND|R6: the held next-turn state's words
text.usage.unconfirmed|UNBOUND|R6: an uncertain operation's words
route.control_sessions|/agents/{agent}/control-sessions|amendment 27; not in draft 93e19e73
route.controls|/runtime/sessions/{session}/controls|Brisket; not in draft 93e19e73
route.control_receipts|/runtime/sessions/{session}/control-receipts|Brisket; not in draft 93e19e73
route.usage|/agents/{agent}/usage|main openapi_table.rs:246
route.turn_act|UNBOUND|R5: the act that gives a managed session a turn; Brisket names
EOF
)
bind() { printf '%s\n' "$BIND" | awk -F'|' -v k="$1" '$1==k {print $2}'; }
unbound=$(printf '%s\n' "$BIND" | awk -F'|' '$2=="UNBOUND" {printf "%s ", $1}')
if [ -n "$unbound" ]; then
  echo "END stage=b result=REFUSED reason=unbound names=[${unbound% }] writes=0"; exit 2
fi

ab() { agent-browser --session "$SESSION" "$@"; }

# One exchange through the signed-in browser page, so no token or cookie reaches this shell. GETs
# are reads; anything else is a write and is logged before it is sent.
api() {
  local method=$1 path=$2 body=${3:-} name=$4 status script
  if [ "$method" != GET ]; then
    printf '%s\t%s\t%s\t%s\t' "$(date '+%H:%M:%S')" "$method" "$path" \
      "$(printf '%s' "$body" | /usr/bin/python3 -c 'import json,sys; print(json.load(sys.stdin).get("operation",""))')" >> "$B/writes.tsv"
  fi
  script=$(/usr/bin/python3 - "$method" "$path" "$body" <<'PY'
import json, sys
method, path, body = sys.argv[1:4]
init = {"method": method, "credentials": "same-origin", "headers": {"accept": "application/json"}}
if body:
    init["headers"]["content-type"] = "application/json"
    init["body"] = body
print("fetch(%s,%s).then(async r=>JSON.stringify({status:r.status,body:await r.text()}))"
      % (json.dumps("/api" + path), json.dumps(init)))
PY
)
  status=$(ab eval "$script" 2>>"$TRACE" | $JUDGE answer 3>"$B/$name.json")
  [ "$method" != GET ] && printf '%s\n' "$status" >> "$B/writes.tsv"
  note "$method $path -> $status ($name)"
  printf '%s' "$status"
}
op() { /usr/bin/python3 -c 'import secrets; print("op-" + secrets.token_hex(16))'; }

# Waits for a parsed condition, re-reading every 5 s until WAIT seconds. Running out is reported
# INCOMPLETE as not observed, never as a pass or a product failure.
until_seen() {
  local deadline=$(( $(date +%s) + WAIT )) check=$1
  while [ "$(date +%s)" -lt "$deadline" ]; do
    eval "$check" && return 0
    /bin/sleep 5
  done
  return 1
}

runner_pid() { pgrep -x lys-runner | head -1; }
harness_children() { local r; r=$(runner_pid); [ -n "$r" ] && pgrep -P "$r" | sort; }

# Process evidence for one launched child: the runner is its parent, fds 0 and 1 are pipes, it has
# no terminal, and no fd 0 to 2 names a tty.
process_checks() {
  local pid=$1 runner ppid tty kinds ttys
  runner=$(runner_pid)
  ppid=$(ps -o ppid= -p "$pid" | tr -d ' ')
  tty=$(ps -o tty= -p "$pid" | tr -d ' ')
  kinds=$(lsof -a -p "$pid" -d 0,1 -F t 2>/dev/null | grep '^t' | sort -u | tr '\n' ',')
  ttys=$(lsof -a -p "$pid" -d 0-2 -F n 2>/dev/null | grep -c '^n/dev/tty')
  note "process pid=$pid ppid=$ppid runner=$runner tty=$tty fd01=$kinds tty_fds=$ttys"
  [ -n "$runner" ] && [ "$ppid" = "$runner" ] && [ "$tty" = '??' ] && [ "$kinds" = 'tPIPE,' ] && [ "$ttys" = 0 ]
}

teardown() {
  local id goal
  id=$(cat "$B/agent.id" 2>/dev/null)
  goal=$(cat "$B/goal.id" 2>/dev/null)
  [ -n "$id" ] || { note "teardown: no agent was registered"; return; }
  [ -n "$goal" ] && api POST "/goals/$goal/active" "{\"operation\":\"$(op)\",\"active\":false}" teardown-goal >/dev/null
  api POST "/network/machines/$MACHINE/agents" "{\"operation\":\"$(op)\",\"agent\":\"$id\",\"allow\":false}" teardown-admission >/dev/null
  api POST "/identities/$id/transitions" "{\"operation\":\"$(op)\",\"transition\":\"retire\",\"reason\":\"BOX 20 qualification ended\"}" teardown-retire >/dev/null
  api GET "/agents/$id/runtime/sessions" '' teardown-sessions >/dev/null
  cut -f2-5 "$B/writes.tsv" > "$B/records.txt"
  live=$(harness_children | comm -12 - "$B/children-ours.txt" 2>/dev/null | grep -c .)
  [ "$live" = 0 ] && leg nothing-left-running PASS || leg nothing-left-running "FAIL:$live"
}

finish() {
  teardown
  result=PASS
  [ -n "$INCOMPLETE" ] && result=INCOMPLETE
  [ -n "$FAILED" ] && result=FAIL
  echo "END stage=b result=$result writes=$(grep -c . "$B/writes.tsv") failed=[${FAILED# }] incomplete=[${INCOMPLETE# }] legs=[${LEGS# }]"
}
trap finish EXIT
: > "$B/children-ours.txt"

# Versions: a harness whose version moved since Stage A or the battery is refused by name.
versions stage-b
REFUSED_HARNESS=''
for earlier in stage-a battery; do
  [ -s "$OUT/versions-$earlier.txt" ] || { leg "versions-$earlier" "INCOMPLETE:not-recorded"; continue; }
  $JUDGE drift "$OUT/versions-$earlier.txt" "$OUT/versions-stage-b.txt" | while read -r kind adapter rest; do
    [ "$kind" = drift ] && echo "$adapter"
  done >> "$B/drifted.txt"
done
for adapter in claude-code codex; do
  if grep -qx "$adapter" "$B/drifted.txt" 2>/dev/null; then
    REFUSED_HARNESS="$REFUSED_HARNESS $adapter"; leg "drift-$adapter" "FAIL:version-changed-refused-by-name"
  elif ! awk -v a="$adapter" -v v="$(awk -v a="$adapter" '$1==a {print $2}' "$OUT/versions-stage-b.txt")" \
      '$1==a && $2==v {found=1} END {exit !found}' "$OUT/stage-a/pins.txt"; then
    REFUSED_HARNESS="$REFUSED_HARNESS $adapter"; leg "pinned-$adapter" "FAIL:not-proved-in-stage-a"
  fi
done

guard || { leg guard "INCOMPLETE:refused"; exit 0; }

# Sign in through Waffles' helper. The script never reads the password file or a cookie.
# It prints one "SIGNIN END: session <name> at <url>" line (Waffles 0452d9da); that line, then an
# administrator read, is the verdict, never its exit code.
"$HELPER" "$SESSION" > "$B/signin.out" 2>&1
cat "$B/signin.out" >> "$TRACE"
grep -qE "^SIGNIN END: session $SESSION at " "$B/signin.out" || { leg signed-in "FAIL:no-signin-end"; exit 0; }
[ "$(api GET /directory/people '' signed-in)" = 200 ] && leg signed-in PASS || { leg signed-in FAIL; exit 0; }

# 1. Register qual-box20 through the Add an agent screen.
ab open "$BASE$(bind screen.add_agent)" >> "$TRACE" 2>&1
ab find label "$(bind label.add.name)" fill "$AGENT" >> "$TRACE" 2>&1
ab find label "$(bind label.add.computer)" select "$MACHINE" >> "$TRACE" 2>&1
ab find label "$(bind label.add.requires_controls)" check >> "$TRACE" 2>&1
ab find label "$(bind label.add.harness)" select claude-code >> "$TRACE" 2>&1
ab find role button click --name "$(bind label.add.submit)" >> "$TRACE" 2>&1
printf '%s\tscreen\t/agents/new\tadd-and-run\t-\n' "$(date '+%H:%M:%S')" >> "$B/writes.tsv"
# Add an agent replaces the page with /file/{agent} once the agent is added (AddAgent.tsx:129,139).
until_seen 'ab get url 2>/dev/null | grep -qE "/file/[^/?#]+"' && leg registered PASS || leg registered "INCOMPLETE:not-observed"
ab get url 2>>"$TRACE" | sed -nE 's#.*/file/([^/?#]+).*#\1#p' | head -1 > "$B/agent.id"
ID=$(cat "$B/agent.id")
[ -n "$ID" ] || { leg agent-id "FAIL:not-found"; exit 0; }

# 2. Threshold and goal with its reminder through the Usage screen; both persist after reload.
usage="$BASE$(bind screen.usage | sed "s/{agent}/$ID/")"
ab open "$usage" >> "$TRACE" 2>&1
ab find label "$(bind label.usage.threshold)" fill 60 >> "$TRACE" 2>&1
ab find label "$(bind label.usage.threshold_action)" select compact >> "$TRACE" 2>&1
ab find role button click --name "$(bind label.usage.threshold_save)" >> "$TRACE" 2>&1
ab find label "$(bind label.goal.words)" fill "qual-box20 reminder check" >> "$TRACE" 2>&1
ab find label "$(bind label.goal.deadline)" fill "$(date -v+1d '+%Y-%m-%dT%H:%M')" >> "$TRACE" 2>&1
ab find label "$(bind label.goal.reminder)" check >> "$TRACE" 2>&1
ab find role button click --name "$(bind label.goal.save)" >> "$TRACE" 2>&1
printf '%s\tscreen\t%s\tthreshold+goal\t-\n' "$(date '+%H:%M:%S')" "$(bind screen.usage)" >> "$B/writes.tsv"
ab reload >> "$TRACE" 2>&1
ab snapshot -c > "$B/usage-after-reload.txt" 2>>"$TRACE"
[ "$(api GET "/budgets/agent/$ID" '' budget)" = 200 ] && grep -q '60' "$B/budget.json" \
  && leg threshold-persists PASS || leg threshold-persists FAIL
[ "$(api GET "/agents/$ID/goals" '' goals)" = 200 ] && $JUDGE get "$B/goals.json" goals.0.id > "$B/goal.id" \
  && leg reminder-persists PASS || leg reminder-persists FAIL

# 3 and 4. Each harness: managed start, boundary delivery, crossing, compaction, stop.
for adapter in claude-code codex; do
  case " $REFUSED_HARNESS " in *" $adapter "*) leg "$adapter" "FAIL:refused-by-name"; continue ;; esac
  guard || { leg "$adapter" "INCOMPLETE:guard"; continue; }
  harness_children > "$B/children-before-$adapter.txt"
  start=$(api POST "/agents/$ID/start-command" "{\"operation\":\"$(op)\",\"machine\":\"$MACHINE\"}" "start-$adapter")
  [ "$start" = 200 ] && leg "start-$adapter" PASS || { leg "start-$adapter" "FAIL:$start"; continue; }
  until_seen "harness_children | comm -13 '$B/children-before-$adapter.txt' - | grep -q ."
  pid=$(harness_children | comm -13 "$B/children-before-$adapter.txt" - | head -1)
  echo "$pid" >> "$B/children-ours.txt"; sort -o "$B/children-ours.txt" "$B/children-ours.txt"
  [ -n "$pid" ] && process_checks "$pid" && leg "pipes-$adapter" PASS || leg "pipes-$adapter" FAIL
  [ "$(api GET "/agents/$ID$(bind route.control_sessions | sed 's#^/agents/{agent}##')" '' "sessions-$adapter")" = 200 ] \
    && S=$($JUDGE get "$B/sessions-$adapter.json" sessions.0.id) || S=''
  [ -n "$S" ] || { leg "session-$adapter" "FAIL:no-control-session"; continue; }
  receipts="$(bind route.control_receipts | sed "s/{session}/$S/")"
  api POST "$(bind route.turn_act | sed "s/{session}/$S/;s/{agent}/$ID/")" "{\"operation\":\"$(op)\"}" "turn-$adapter" >/dev/null
  until_seen "[ \"\$(api GET '$receipts' '' 'receipts-$adapter')\" = 200 ] && grep -q '\"reminder\"' '$B/receipts-$adapter.json'" \
    && leg "reminder-$adapter" PASS || leg "reminder-$adapter" "INCOMPLETE:not-observed"
  ab open "$usage" >> "$TRACE" 2>&1; ab snapshot -c > "$B/usage-held-$adapter.txt" 2>>"$TRACE"
  grep -qF "$(bind text.usage.held)" "$B/usage-held-$adapter.txt" && leg "held-shown-$adapter" PASS || leg "held-shown-$adapter" "INCOMPLETE:not-shown"
  case $adapter in claude-code) kind=compact_boundary ;; codex) kind=contextCompaction ;; esac
  until_seen "[ \"\$(api GET '$receipts' '' 'receipts-$adapter')\" = 200 ] && grep -q '$kind' '$B/receipts-$adapter.json'" \
    && leg "compaction-$adapter" PASS || leg "compaction-$adapter" "INCOMPLETE:not-observed"
  [ "$(api GET "/agents/$ID/usage" '' "usage-$adapter")" = 200 ] && leg "usage-$adapter" PASS || leg "usage-$adapter" FAIL
  api POST "/agents/$ID/stop" "{\"operation\":\"$(op)\",\"reason\":\"BOX 20 qualification stop\"}" "stop-$adapter" >/dev/null
  until_seen "! ps -p '$pid' > /dev/null" && leg "exit-$adapter" PASS || leg "exit-$adapter" "FAIL:still-running"
  [ "$adapter" = claude-code ] && api POST "/identities/$ID/transitions" "{\"operation\":\"$(op)\",\"transition\":\"reinstate\",\"reason\":\"BOX 20 qualification: next harness\"}" reinstate >/dev/null
done
exit 0
