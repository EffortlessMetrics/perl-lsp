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

Measurement wiring is clear. Follow [`parser.md`](parser.md#raw-failure-buckets) for capability work only when the generated parser status lists a nonzero raw failure bucket. If parser status lists `none`, do not start parser bucket work from stale context; refresh the Linux corpus receipt or move to the next provider or real-workspace trust lane.
