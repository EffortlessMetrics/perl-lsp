# Parser Accuracy Next

Source: `target/metrics/parser_accuracy.json`

Denominator: 50 fixtures / 29 families; 139 scored lines; 117 scored symbols.

Failure packets: 0 active.

Pointer: no active failure packets.

## Next Measurement Gaps

| Metric | Reason | Suggested PR |
|---|---|---|
| none | n/a | n/a |

Use the measurement gap table only when there are no active failure packets. Keep each measurement gap in its own PR and regenerate this file after a lane lands.

## Capability Handoff

Measurement wiring is clear. Follow [`parser.md`](parser.md#parser-failure-worklist-clustered) for capability work; take the largest nonzero parser failure cluster as the next parser lane and keep it separate from measurement-only work.
