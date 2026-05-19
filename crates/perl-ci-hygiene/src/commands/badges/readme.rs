use color_eyre::eyre::Result;
use regex::Regex;
use std::sync::LazyLock;

static VS_MARKETPLACE_INSTALLS_BADGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"VS%20Marketplace-(\d+)%20installs").unwrap_or_else(|error| {
        unreachable!("VS_MARKETPLACE_INSTALLS_BADGE_RE is a known-good static pattern: {error}")
    })
});

static VS_MARKETPLACE_INSTALLS_BADGE_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?s)<!-- perl-lsp:vs-marketplace-installs-badge:start -->.*?<!-- perl-lsp:vs-marketplace-installs-badge:end -->",
    )
    .unwrap_or_else(|error| {
        unreachable!("VS_MARKETPLACE_INSTALLS_BADGE_BLOCK_RE is a known-good static pattern: {error}")
    })
});

pub(super) fn stale_installs_value(content: &str) -> Option<String> {
    VS_MARKETPLACE_INSTALLS_BADGE_RE
        .captures(content)
        .and_then(|caps| caps.get(1).map(|found| found.as_str().to_string()))
}

pub(super) fn update_badge_in_content(content: &str, badge_url: &str) -> Result<String> {
    if !VS_MARKETPLACE_INSTALLS_BADGE_BLOCK_RE.is_match(content) {
        return Ok(content.to_string());
    }

    if content.contains("href=\"https://marketplace.visualstudio.com") {
        let replacement = format!(
            "<!-- perl-lsp:vs-marketplace-installs-badge:start -->\n  <a href=\"https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs\"><img src=\"{}\" alt=\"VS Marketplace installs\" /></a>\n  <!-- perl-lsp:vs-marketplace-installs-badge:end -->",
            badge_url
        );
        return Ok(VS_MARKETPLACE_INSTALLS_BADGE_BLOCK_RE
            .replace_all(content, replacement)
            .into_owned());
    }

    let replacement = format!(
        "<!-- perl-lsp:vs-marketplace-installs-badge:start -->\n[![VS Marketplace Installs (manual)]({})](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs)\n<!-- perl-lsp:vs-marketplace-installs-badge:end -->",
        badge_url
    );
    Ok(VS_MARKETPLACE_INSTALLS_BADGE_BLOCK_RE.replace_all(content, replacement).into_owned())
}
