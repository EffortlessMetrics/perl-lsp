# Perl Community Sentiment: LSP Adoption (2026)

*Explore agent research report. Data current as of March 2026.*

*Source: Perl IDE Survey 2025, https://survey.perlide.org/results/2025 — 602 respondents*

---

## Headline Finding

Despite six years of Perl LSP implementations, 78% of Perl developers use no language server at all.

---

## Survey Data (n=602)

### LSP Adoption

| Status | Share |
|--------|-------|
| No LSP | 78% |
| Use LSP | 22% |

### LSP Tool Usage (among the 22% who use one)

| Tool | Approximate Users (of 602) |
|------|---------------------------|
| PerlNavigator | ~66 |
| Perl::LanguageServer | ~44 |
| PLS | ~8 |

Note: These are derived counts from 22% of 602. Rounding applies.

### IDE Distribution

| IDE | Share |
|-----|-------|
| Vim | 28.4% |
| VS Code | 21.6% |
| Emacs | 14.1% |
| Neovim | 7.0% |
| Other | ~29% |

---

## Interpretation

### The Adoption Gap

The 78% non-adoption rate is striking given that PerlNavigator has been available since 2022 and Perl::LanguageServer since 2018. This is not a supply problem — tools exist. The gap suggests:

1. **Awareness**: Many Perl developers may not know LSP tooling is available or mature enough
2. **Editor fragmentation**: Vim (28.4%) and Emacs (14.1%) users have different LSP integration workflows than VS Code users; combined they represent 42.5% of the respondent base
3. **Tool quality**: The leading tool by installs (Perl::LanguageServer, 293K) has been stale since December 2023, which may suppress adoption among developers who tried it
4. **Community norms**: Perl's culture of "editor as personal environment" may reduce uptake of opinionated tooling

### The Neovim Gap

Neovim at 7% has native LSP support built in (since v0.5, 2021). Yet LSP adoption among this segment is not separable from the 22% overall figure. This is a segment that should be early adopters of LSP.

### IDE Mix Implications

VS Code at 21.6% is the strongest target for LSP distribution — VS Code's extension model is the most accessible path for new users. But Vim (28.4%) being the top editor means any LSP strategy must account for `coc.nvim` and similar setups.

---

## Conference Landscape

### TPRC 2026

- **When**: June 25-29, 2026
- **Where**: Greenville, SC
- **Status**: Soliciting tooling talks
- **Gap**: No dedicated LSP track despite 6+ years of availability

### PTS 2026 (Perl Toolchain Summit)

- **Attendees**: 33 maintainers
- **Focus**: CPAN infrastructure
- **Gap**: LSP tooling not in current agenda despite direct relevance to developer workflows

---

## Narrative Frame

"Despite six years of Perl LSP implementations, 78% still don't use them."

This is the key story. The market is not saturated — it is underpenetrated. The audience that would benefit most (the 78%) has not been reached by any of the existing tools. The stale leader (Perl::LanguageServer) may be actively suppressing trust in the category.

perl-lsp's position: the first actively maintained, native-parser-backed Perl LSP with full CPAN corpus coverage — entering a market where adoption is low not because users rejected LSP, but because no tool has successfully converted them.

---

## Strategic Implications

| Observation | Action |
|-------------|--------|
| 78% non-adoption | Messaging should explain the category, not just differentiate perl-lsp |
| Vim is #1 editor | Ensure Vim/Neovim integration is documented and tested |
| TPRC soliciting talks | Submit a tooling talk with survey data and live demo |
| No LSP track at conferences | Opportunity to be first; propose a BOF or session |
| PLS stale | Target PLS users (9,709 installs) with direct migration path |
| Perl::LanguageServer stale | Large user base (293K installs) with no maintained upgrade path |

---

*Source: Perl IDE Survey 2025 (https://survey.perlide.org/results/2025), 602 respondents. Explore agent findings, March 2026.*
