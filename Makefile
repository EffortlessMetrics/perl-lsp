.PHONY: plan execute loop finalize docs merge finalize-lane labels

plan:
	@echo "Planning full flow (no execution) ..."
	@echo "1) pr-initial-reviewer (plan)"
	@echo "2) loop (tests → scout → cleanup) (plan)"
	@echo "3) pr-finalize-agent (plan)"
	@echo "4) pr-doc-finalize (plan)"
	@echo "5) pr-merger (plan)"
	@echo "6) pr-finalizer (plan)"

execute:
	@echo "EXECUTION mode enabled for agents via AGENT_EXECUTE=1"
	@AGENT_EXECUTE=1 $(MAKE) loop finalize docs merge finalize-lane

loop:
	@bash scripts/agents/run-tests.sh
	@echo "→ Next: context-scout (plan), pr-cleanup (plan), then re-run tests (gated)"

finalize:
	@echo "→ pr-finalize-agent (local gates): plan/execution handled inside agent spec"

docs:
	@echo "→ pr-doc-finalize (Diátaxis + adjacent docs): plan/execution handled inside agent spec"

merge:
	@echo "→ pr-merger (remote via gh): plan/execution handled inside agent spec"

finalize-lane:
	@echo "→ pr-finalizer (lane sync): plan/execution handled inside agent spec"

labels:
	@gh label create 'tests:green'     --color 2ECC71 || true
	@gh label create 'finalize:passed' --color 1F8B4C || true
	@gh label create 'needs-rework'    --color D73A4A || true
	@gh label create 'ready-for-ai'    --color 5319E7 || true
	@gh label create 'area:parser'     --color 0E8A16 || true
	@gh label create 'area:lexer'      --color 0052CC || true
	@gh label create 'area:lsp'        --color 0366d6 || true