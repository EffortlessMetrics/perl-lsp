// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="introduction.html">Introduction</a></li><li class="chapter-item expanded affix "><a href="quick-start.html">Quick Start</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">Getting Started</li><li class="chapter-item expanded "><a href="getting-started/installation.html"><strong aria-hidden="true">1.</strong> Installation</a></li><li class="chapter-item expanded "><a href="getting-started/editor-setup.html"><strong aria-hidden="true">2.</strong> Editor Setup</a></li><li class="chapter-item expanded "><a href="getting-started/first-steps.html"><strong aria-hidden="true">3.</strong> First Steps</a></li><li class="chapter-item expanded "><a href="getting-started/configuration.html"><strong aria-hidden="true">4.</strong> Configuration</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">User Guides</li><li class="chapter-item expanded "><a href="user-guides/lsp-features.html"><strong aria-hidden="true">5.</strong> LSP Features</a></li><li class="chapter-item expanded "><a href="user-guides/workspace-navigation.html"><strong aria-hidden="true">6.</strong> Workspace Navigation</a></li><li class="chapter-item expanded "><a href="user-guides/debugging.html"><strong aria-hidden="true">7.</strong> Debugging with DAP</a></li><li class="chapter-item expanded "><a href="user-guides/troubleshooting.html"><strong aria-hidden="true">8.</strong> Troubleshooting</a></li><li class="chapter-item expanded "><a href="user-guides/known-limitations.html"><strong aria-hidden="true">9.</strong> Known Limitations</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">Architecture</li><li class="chapter-item expanded "><a href="architecture/overview.html"><strong aria-hidden="true">10.</strong> Overview</a></li><li class="chapter-item expanded "><a href="architecture/crate-structure.html"><strong aria-hidden="true">11.</strong> Crate Structure</a></li><li class="chapter-item expanded "><a href="architecture/parser-design.html"><strong aria-hidden="true">12.</strong> Parser Design</a></li><li class="chapter-item expanded "><a href="architecture/lsp-implementation.html"><strong aria-hidden="true">13.</strong> LSP Implementation</a></li><li class="chapter-item expanded "><a href="architecture/dap-implementation.html"><strong aria-hidden="true">14.</strong> DAP Implementation</a></li><li class="chapter-item expanded "><a href="architecture/modern-architecture.html"><strong aria-hidden="true">15.</strong> Modern Architecture</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">Developer Guides</li><li class="chapter-item expanded "><a href="developer/contributing.html"><strong aria-hidden="true">16.</strong> Contributing</a></li><li class="chapter-item expanded "><a href="developer/commands-reference.html"><strong aria-hidden="true">17.</strong> Commands Reference</a></li><li class="chapter-item expanded "><a href="developer/testing-guide.html"><strong aria-hidden="true">18.</strong> Testing Guide</a></li><li class="chapter-item expanded "><a href="developer/test-infrastructure.html"><strong aria-hidden="true">19.</strong> Test Infrastructure</a></li><li class="chapter-item expanded "><a href="developer/api-documentation-standards.html"><strong aria-hidden="true">20.</strong> API Documentation Standards</a></li><li class="chapter-item expanded "><a href="developer/development-workflow.html"><strong aria-hidden="true">21.</strong> Development Workflow</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">LSP Development</li><li class="chapter-item expanded "><a href="lsp/implementation-guide.html"><strong aria-hidden="true">22.</strong> LSP Implementation Guide</a></li><li class="chapter-item expanded "><a href="lsp/providers-reference.html"><strong aria-hidden="true">23.</strong> LSP Providers Reference</a></li><li class="chapter-item expanded "><a href="lsp/feature-implementation.html"><strong aria-hidden="true">24.</strong> Feature Implementation</a></li><li class="chapter-item expanded "><a href="lsp/cancellation-system.html"><strong aria-hidden="true">25.</strong> Cancellation System</a></li><li class="chapter-item expanded "><a href="lsp/error-handling.html"><strong aria-hidden="true">26.</strong> Error Handling</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">Advanced Topics</li><li class="chapter-item expanded "><a href="advanced/performance-guide.html"><strong aria-hidden="true">27.</strong> Performance Guide</a></li><li class="chapter-item expanded "><a href="advanced/incremental-parsing.html"><strong aria-hidden="true">28.</strong> Incremental Parsing</a></li><li class="chapter-item expanded "><a href="advanced/threading-configuration.html"><strong aria-hidden="true">29.</strong> Threading Configuration</a></li><li class="chapter-item expanded "><a href="advanced/security-development.html"><strong aria-hidden="true">30.</strong> Security Development</a></li><li class="chapter-item expanded "><a href="advanced/mutation-testing.html"><strong aria-hidden="true">31.</strong> Mutation Testing</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">Reference</li><li class="chapter-item expanded "><a href="reference/current-status.html"><strong aria-hidden="true">32.</strong> Current Status</a></li><li class="chapter-item expanded "><a href="reference/roadmap.html"><strong aria-hidden="true">33.</strong> Roadmap</a></li><li class="chapter-item expanded "><a href="reference/milestones.html"><strong aria-hidden="true">34.</strong> Milestones</a></li><li class="chapter-item expanded "><a href="reference/stability.html"><strong aria-hidden="true">35.</strong> Stability Statement</a></li><li class="chapter-item expanded "><a href="reference/upgrading.html"><strong aria-hidden="true">36.</strong> Upgrade Guides</a></li><li class="chapter-item expanded "><a href="reference/error-handling-contracts.html"><strong aria-hidden="true">37.</strong> Error Handling Contracts</a></li><li class="chapter-item expanded "><a href="reference/lsp-missing-features.html"><strong aria-hidden="true">38.</strong> LSP Missing Features</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">DAP (Debug Adapter)</li><li class="chapter-item expanded "><a href="dap/user-guide.html"><strong aria-hidden="true">39.</strong> DAP User Guide</a></li><li class="chapter-item expanded "><a href="dap/implementation.html"><strong aria-hidden="true">40.</strong> DAP Implementation</a></li><li class="chapter-item expanded "><a href="dap/security.html"><strong aria-hidden="true">41.</strong> DAP Security</a></li><li class="chapter-item expanded "><a href="dap/bridge-setup.html"><strong aria-hidden="true">42.</strong> DAP Bridge Setup</a></li><li class="chapter-item expanded "><a href="dap/protocol-schema.html"><strong aria-hidden="true">43.</strong> DAP Protocol Schema</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">CI &amp; Quality</li><li class="chapter-item expanded "><a href="ci/overview.html"><strong aria-hidden="true">44.</strong> CI Overview</a></li><li class="chapter-item expanded "><a href="ci/local-validation.html"><strong aria-hidden="true">45.</strong> Local Validation</a></li><li class="chapter-item expanded "><a href="ci/test-lanes.html"><strong aria-hidden="true">46.</strong> Test Lanes</a></li><li class="chapter-item expanded "><a href="ci/cost-tracking.html"><strong aria-hidden="true">47.</strong> Cost Tracking</a></li><li class="chapter-item expanded "><a href="ci/debt-tracking.html"><strong aria-hidden="true">48.</strong> Debt Tracking</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">Process &amp; Governance</li><li class="chapter-item expanded "><a href="process/agentic-dev.html"><strong aria-hidden="true">49.</strong> Agentic Development</a></li><li class="chapter-item expanded "><a href="process/lessons.html"><strong aria-hidden="true">50.</strong> Lessons Learned</a></li><li class="chapter-item expanded "><a href="process/casebook.html"><strong aria-hidden="true">51.</strong> Casebook</a></li><li class="chapter-item expanded "><a href="process/documentation-truth.html"><strong aria-hidden="true">52.</strong> Documentation Truth System</a></li><li class="chapter-item expanded "><a href="process/quality-surfaces.html"><strong aria-hidden="true">53.</strong> Quality Surfaces</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">Additional Resources</li><li class="chapter-item expanded "><a href="resources/adr.html"><strong aria-hidden="true">54.</strong> ADRs (Architecture Decision Records)</a></li><li class="chapter-item expanded "><a href="resources/benchmarks.html"><strong aria-hidden="true">55.</strong> Benchmarks</a></li><li class="chapter-item expanded "><a href="resources/forensics.html"><strong aria-hidden="true">56.</strong> Forensics</a></li><li class="chapter-item expanded "><a href="resources/issue-tracking.html"><strong aria-hidden="true">57.</strong> Issue Tracking</a></li><li class="chapter-item expanded "><a href="resources/ga-runbook.html"><strong aria-hidden="true">58.</strong> GA Runbook</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
