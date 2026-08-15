# Durable Evaluations

`docs/eval/` contains repository-owned evaluation procedures intended to be
run again as the relevant agent, workflow, or contract evolves. Group each
procedure by domain, such as `publishing/`.

Every evaluation document must state:

- its concrete goals; and
- the observable expected outcomes for both success and relevant safe-denial
  scenarios.

An evaluation must also identify any prohibited side effects so a future run
can distinguish an effective safety check from an accidental production action.
