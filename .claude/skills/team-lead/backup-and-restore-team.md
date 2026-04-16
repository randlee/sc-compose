# Team Backup and Restore Procedure

Follow this procedure when Step 1 of the team-lead skill detects a session ID
mismatch or a missing `config.json` (i.e., a full team restore is required).

---

## Step 2 — Backup Current State

Always backup before modifying the team:

```bash
atm teams backup sc-compose
# Note the backup path from output, e.g.:
# Backup created: ~/.claude/teams/.backups/sc-compose/<timestamp>
```

Also backup the Claude Code project task list (separate bucket):

```bash
BACKUP_PATH=$(ls -td ~/.claude/teams/.backups/sc-compose/*/ | head -1)
cp -r ~/.claude/tasks/sc-compose/ "$BACKUP_PATH/tasks-cc" 2>/dev/null || true
echo "CC task list backed up to $BACKUP_PATH/tasks-cc"
```

> **Note**: `atm teams backup` captures `~/.claude/tasks/sc-compose/` (ATM sprint
> tasks) but NOT the Claude Code task tools bucket. These are two separate buckets.

---

## Step 3 — Repair or Recreate Team Config

Two cases:

### Case A — `config.json` missing but directory exists (most common)

Reconstruct `config.json` from the latest backup + current session ID:

```python
python3 -c "
import json, os, glob

# Find latest backup
backups = sorted(glob.glob(os.path.expanduser(
    '~/.claude/teams/.backups/sc-compose/*/config.json')))
if not backups:
    raise SystemExit('No backup found — cannot restore')

with open(backups[-1]) as f:
    cfg = json.load(f)

# Stamp current session ID (get from SESSION_ID= in context or atm whoami)
import subprocess
session_id = subprocess.check_output(['atm', 'whoami', '--session-id'],
    text=True).strip()
cfg['leadSessionId'] = session_id

# Clear stale tmuxPaneId for team-lead
for m in cfg['members']:
    if m['name'] == 'team-lead':
        m['tmuxPaneId'] = ''

out = os.path.expanduser('~/.claude/teams/sc-compose/config.json')
with open(out, 'w') as f:
    json.dump(cfg, f, indent=2)
print('Restored from:', backups[-1])
print('Members:', [m['name'] for m in cfg['members']])
print('leadSessionId:', cfg['leadSessionId'])
"
```

If `atm whoami --session-id` is unavailable, supply the SESSION_ID manually
(visible in the `SessionStart` hook output at the top of context).

### Case B — Directory missing entirely

```bash
# 1. Clear any active team context in this session
TeamDelete  # tool call — may say "No team name found", that is OK

# 2. Create fresh team
TeamCreate(team_name="sc-compose", description="sc-compose development team", agent_type="team-lead")
# Verify team_name in response is "sc-compose" — stop if it is not

# 3. Restore members and inboxes from backup
LATEST=$(ls -td ~/.claude/teams/.backups/sc-compose/*/ | head -1)
atm teams restore sc-compose --from "$LATEST"
# Expected: N member(s) added, N inbox file(s) restored
```

---

## Step 4 — Verify and Prune Members

```bash
atm members
```

Expected members: `team-lead`, `quality-mgr`, `comp`.
Remove unexpected members if present (until `atm teams remove-member` ships):

```python
python3 -c "
import json
path = '/Users/randlee/.claude/teams/sc-compose/config.json'
with open(path) as f: cfg = json.load(f)
keep = ['team-lead', 'quality-mgr', 'comp']
cfg['members'] = [m for m in cfg['members'] if m['name'] in keep]
with open(path, 'w') as f: json.dump(cfg, f, indent=2)
print('Members:', [m['name'] for m in cfg['members']])
"
```

---

## Step 5 — Restore Claude Code Task List

```bash
BACKUP_PATH=$(ls -td ~/.claude/teams/.backups/sc-compose/*/ | head -1)
if [ -d "$BACKUP_PATH/tasks-cc" ]; then
  cp "$BACKUP_PATH/tasks-cc/"*.json ~/.claude/tasks/sc-compose/ 2>/dev/null || true
  MAX_ID=$(ls ~/.claude/tasks/sc-compose/*.json 2>/dev/null \
    | xargs -I{} basename {} .json \
    | sort -n | tail -1)
  [ -n "$MAX_ID" ] && echo -n "$MAX_ID" > ~/.claude/tasks/sc-compose/.highwatermark
  echo "Task list restored. Highwatermark: $MAX_ID"
else
  echo "No tasks-cc/ in backup — task list not restored."
fi
```

> The Claude Code UI task panel will not show restored tasks until one task is
> created via `TaskCreate`. Create a real task to trigger the panel refresh.

> **Known bug**: `atm teams restore` sets `.highwatermark` to `min_id - 1`
> instead of `max_id`. The script above corrects this manually.

---

## Step 6 — Verify Team Health

```bash
atm members          # confirm expected members
atm inbox            # check for unread messages
atm gh pr list       # open PRs and CI status
```

---

## Step 7 — Read Project Context

1. Read `docs/project-plan.md` — focus on current phase and open tasks
2. Check `TaskList` — recreate pending tasks via `TaskCreate` if list is empty
3. Output a concise project summary:
   - Current phase and status
   - Open PRs
   - Active teammates and their last known task
   - Next sprint(s) ready to execute

---

## Step 8 — Notify Teammates

```bash
atm send comp "New session (session-id: <SESSION_ID>). Team sc-compose restored. Please ACK and confirm status."
atm send quality-mgr "New session (session-id: <SESSION_ID>). Team sc-compose restored. Please ACK and confirm status."
```

If no response within ~60s, nudge comp via tmux (comp is a Codex agent and
does not receive ATM push — must be poked to poll inbox):

```bash
tmux list-panes -a -F '#{session_name}:#{window_index}.#{pane_index} #{pane_title}'
tmux send-keys -t <pane-id> "" && sleep 0.5 && \
tmux send-keys -t <pane-id> "You have unread ATM messages." Enter
```

---

## Common Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| `config.json` missing, dir exists | Config deleted or corrupted | Use Case A in Step 3 |
| `TeamCreate` returns random name | `~/.claude/teams/sc-compose` still exists | `rm -rf ~/.claude/teams/sc-compose` then retry |
| `TeamDelete` says "No team name found" | Fresh session, no active team context | Expected — proceed |
| `atm teams restore` fails "team not found" | Directory missing | Create dir first or use `TeamCreate` (Case B) |
| `TaskList` returns empty after restore | Highwatermark mismatch | Set manually + create one task via `TaskCreate` |
| `atm send` fails "Agent not found" | Member lost after restore overwrite | `atm teams add-member sc-compose <name> ...` |
| Self-send (team-lead → team-lead) | Teammate wrong `ATM_IDENTITY` | Relaunch with `ATM_IDENTITY=<correct-name>` |
