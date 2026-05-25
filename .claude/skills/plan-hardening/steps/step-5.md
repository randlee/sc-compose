# Step 5 — Consistency Hardening (`comp`)

## Execute

**1. Render the message**

```bash
sc-compose render \
  --root .claude/skills/plan-hardening \
  --file 03-consistency-hardening.xml.j2 \
  --var-file /tmp/plan-hardening-vars.json \
  --output /tmp/step-5-message.xml
```

The vars file or rendered task must include `step-4` fenced JSON as the
required input payload.
It must also carry current round metadata:
- `round_id`
- `round_index`
- `replay_nonce`
- `reviewed_commit`
- `previous_reviewed_commit`
- `findings_hash`

**2. Send to `comp`**

```bash
atm send comp --stdin < /tmp/step-5-message.xml
```

**3. Check the response**

Read the `comp` response and confirm it contains fenced JSON.
The expected output shape is specified inside
`03-consistency-hardening.xml.j2`.
Do not proceed to Step 6 until that fenced JSON is present and well formed.
If the response is incomplete or malformed, send a correction request to
`comp` immediately.
Save the extracted fenced JSON to `/tmp/step-5.json`.

**4. Route by status**

- `PASS` -> proceed to Step 6
- `FAIL` -> re-render and re-send Step 5 to `comp`
- if `comp` ACKs but responds as though the same already-fixed round is
  being replayed, increment `round_index`, update `round_id`, refresh
  `replay_nonce` with the current UTC timestamp, and re-render before
  re-sending
- if Step 5 returns `FAIL` three times without converging, escalate to the
  user before continuing

## Hard stops

- `step-4` fenced JSON from the Step 4 response is missing or malformed: do
  not advance; send a correction request immediately and identify the missing
  or malformed fields explicitly
- fenced JSON is missing or malformed: do not advance; send a correction
  request immediately and identify the missing or malformed fields explicitly
- Step 5 has returned `FAIL` three times without converging: do not advance;
  escalate to the user before continuing
