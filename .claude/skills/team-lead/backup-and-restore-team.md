---
name: backup-and-restore-team
version: 0.2.0
description: Procedure for backing up and restoring an ATM team. Referenced by the team-lead skill when a session ID mismatch is detected.
---
# Team Backup And Restore Procedure

Follow this procedure when Step 1 of the `team-lead` skill detects a session id
mismatch and a full team restore is required. This is the startup or `clear`
path where the live `SESSION_ID` changed and no longer matches
`leadSessionId`.

Do not use this procedure for same-session compaction or resume when the
session id still matches. Verify teammate routing first before using this
destructive restore path.

## Step 2 — Backup Current State

Always back up before modifying the team:

```bash
atm teams backup sc-compose
```

Also back up the Claude Code project task list separately:

```bash
BACKUP_PATH=$(ls -td ~/.claude/teams/.backups/sc-compose/*/ | head -1)
cp -r ~/.claude/tasks/sc-compose/ "$BACKUP_PATH/tasks-cc"
echo "CC task list backed up to $BACKUP_PATH/tasks-cc"
```

Note: back up the `~/.claude/tasks/sc-compose/` task bucket explicitly even if
`atm teams backup` also captures team state. Do not assume task history is
restored unless this directory is copied back.

## Step 3 — Clear Stale Team State

```text
TeamDelete
```

Then remove the stale team directory so the next create uses the correct name:

```bash
rm -rf ~/.claude/teams/sc-compose
```

If `TeamDelete` already removed the directory, the `rm -rf` is harmless.

## Step 4 — Create Team

```text
TeamCreate(team_name="sc-compose", description="sc-compose development team", agent_type="team-lead")
```

Verify that the returned team name is exactly `sc-compose`. If it is not, stop.

Note: if communications are broken after compaction or resume, confirm the
failure first and avoid this destructive backup/delete/restore flow unless a
lighter routing repair is impossible.

## Step 5 — Restore Team Members And Inboxes

```bash
atm teams restore sc-compose --from ~/.claude/teams/.backups/sc-compose/<timestamp>
```

Verify members:

```bash
atm members
```

If unexpected ghost members exist, trim the config manually:

```bash
python3 -c "
import json
path = '/Users/randlee/.claude/teams/sc-compose/config.json'
with open(path) as f:
    cfg = json.load(f)
keep = ['team-lead', 'comp', 'quality-mgr']
cfg['members'] = [m for m in cfg['members'] if m['name'] in keep]
with open(path, 'w') as f:
    json.dump(cfg, f, indent=2)
print('Members:', [m['name'] for m in cfg['members']])
"
```

Adjust the `keep` list if additional named teammates are intentionally active.

## Step 6 — Restore Claude Code Task List

```bash
BACKUP_PATH=$(ls -td ~/.claude/teams/.backups/sc-compose/*/ | head -1)
if [ -d "$BACKUP_PATH/tasks-cc" ]; then
  mkdir -p ~/.claude/tasks/sc-compose
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

The Claude Code UI task panel may not show restored tasks until one task is
created through the task tool.

## Step 7 — Verify Team Health

```bash
atm members
atm read team-lead --unread-only --no-mark --json
gh pr list
```

Communication verification is also mandatory:
1. `atm send quality-mgr "team restore verification ping"` to prove teammate
   routing works.
2. `atm send comp "team restore verification ping"` and confirm the Codex-side
   nudge or mailbox poll path still works.

## Step 8 — Read Project Context

1. Read `docs/project-plan.md`.
2. Recreate pending tasks if the task list is empty.
3. Output a concise project summary:
   - current phase and status
   - open PRs
   - active teammates and their last known task
   - next sprint or sprints ready to execute

## Step 9 — Notify Teammates

```bash
atm send comp "New session (session-id: <SESSION_ID>). Team sc-compose restored. Please acknowledge and confirm status."
```

If no response arrives within about 60 seconds, nudge via tmux. Preferred
structured nudge payload when task metadata is available:

```text
<atm><action>read atm</action><action>ack <TASK-ID></action><action>execute assigned task</action><when idle="immediate" busy="after-current-task"/><console announce="concise" pause="false"/></atm>
```

Fallback plain-text nudge:

```bash
tmux list-panes -a -F '#{session_name}:#{window_index}.#{pane_index} #{pane_title}'
tmux send-keys -t <pane-id> "read atm for task <TASK-ID> and complete it before stopping" Enter
```

## Common Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| `TeamCreate` returns random name | `~/.claude/teams/sc-compose` still exists | remove the directory and retry |
| `TeamDelete` says no team name found | fresh session with no active team context | expected, proceed |
| task list looks empty after restore | highwatermark mismatch or UI stale state | set `.highwatermark`, then create one real task |
| `atm send` fails with agent not found | member missing after restore | add the member back to the team |
| self-send or wrong identity routing | teammate launched with wrong `ATM_IDENTITY` | relaunch with the correct identity |
