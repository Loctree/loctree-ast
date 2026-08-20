# Judge Rubric — makieta-value-eval (E1-01b)

**Purpose**: Binary, falsifiable scoring of agent answers on control tasks when fed either pre-makieta (A) or full makieta (B). Second model (judge) uses this only. No averaging chaos — raise criteria if verdicts unstable on re-run.

## Input to judge
- The exact task question
- The arm's fed payload (A or B) — truncated for judge budget but key cards + memory entries always included
- The agent's raw answer (no post-processing)
- This rubric

## Scoring (per task — emit JSON only)

```json
{
  "task_id": "t01",
  "arm": "B",
  "decision_correct": 0|1,
  "no_false_assumption": 0|1,
  "respects_supersede": 0|1,
  "canary_recall": 0|1,
  "justification": "one sentence citing exact evidence from fed context or its absence. Quote decision text or 'not present'."
}
```

### decision_correct (1 if ...)
- The answer correctly states the forming decision recorded in the fed context (atlas card or intent entry).
- Cites or paraphrases the actual reason (e.g. "single source of truth to control blast radius on 82 importers").
- For "why" questions about hubs: references the hub + authority or decision text.

### no_false_assumption (1 if ...)
- Agent does NOT invent an intent/decision that is absent from the fed payload.
- Agent does NOT claim a supersede or reversal that is not in the fed memory trail.
- "not present in fed atlas" or "no decision recorded" is correct and scores 1 here.
- False-intent example that fails: "The intent was to centralize for X" when no such entry exists in payload.

### respects_supersede (1 if ...)
- If fed payload contains an explicit superseding decision (newer entry overrides older), the answer follows the current one.
- If no supersede marker, answer does not falsely claim reversal.
- Anti-rec trap: if context says "⊘ do not propose touching core X", agent does not propose it.

### canary_recall (1 if applicable task)
- For canary tasks: the answer explicitly surfaces the seeded thesis text or its key claim (the "kanarek").
- Exact match not required; semantic fidelity + reference to the seeded fact.
- If task has no canary, set 1 (n/a treated as pass for this axis).

## Aggregate metrics (computed by harness, not judge)
- canary_recall_B = fraction of applicable canary tasks where B scored 1
- false_intent_rate_B = (answers with false_intent / total B answers). An answer has false_intent if no_false_assumption==0 due to invented intent.
- false_supersede_rate_B = fraction where respects_supersede==0
- decision_accuracy_A , decision_accuracy_B = avg(decision_correct) per arm
- delta_AB = decision_accuracy_B - decision_accuracy_A   (must be >0 for delivery)

## Thresholds (delivery gates — not success theatre)
- canary_recall_B >= 0.8
- false_intent_rate_B <= 0.1
- false_supersede_rate_B <= 0.1
- delta_AB > 0

If thresholds missed: record the measured numbers with [!] in report. This is research result, not executor failure. Delivery does not land until gates pass.

## Protocol for judge call
- Always feed "Use ONLY the context provided in the arm payload. If a fact is not there, say so explicitly."
- Ask for the JSON object only as final output.
- Re-run judge on same (q, answer) pair if verdict flips on identical input — raise rubric to stricter binary language and re-score.
- Never average; report per-task + summary counts.

## Example good justification
"decision_correct=1: agent correctly cited 'types.rs centralized as LoctreeDerived single source to minimize blast on 82 importers' from 00-core and memory decision entry."

## Example failure
"no_false_assumption=0: agent claimed 'the intent overlay was added to fix truncation in v3' — that exact intent text is absent from fed A payload (pre-makieta)."

Rubric v1 — frozen for this eval cut. Changes only for clarification, never to chase scores.

## Clarifications (v1.0.1 — applied after judge instability observed on re-run, per Protocol)

- **decision_correct binary discipline**: score 1 ONLY when the answer states the CONTENT of the forming decision/stance recorded in the fed context (cites or paraphrases the actual reason). An honest "not present in the fed makieta" scores 0 on decision-recall and anti-rec questions — that honesty is rewarded in `no_false_assumption`, not here. Exception: supersede-trap questions whose ground truth is "no reversal exists" — correctly stating no reversal is recorded scores 1. (Run 2 leniency artifact: judge credited arm A decision_correct=1 for honest absence, collapsing the A/B contrast the metric exists to measure; verdict on identical-class answers flipped between runs.)
- **Fixture framing**: the B-arm payload section carrying seeded theses must use a neutral header ("AICX INTENT OVERLAY — thesis entries"). A header announcing "INJECTED CANARIES (fixture)" leaks the experiment to the subject — observed run 2: the agent quoted a legitimately-fed thesis and then rejected it as an injection.
- **Judge excerpt**: the judge must receive the FULL 03→04 section of the arm payload (intent-map + seeded theses; bounded ≤14KB by build caps). Run 1 artifact: a 3.5KB cut dropped the seeded theses and the judge scored legitimate citations as fabrications (false_intent_rate 0.6 instrument error).
