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
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="introduction.html">Introduction</a></li><li class="chapter-item expanded affix "><li class="part-title">Getting Started</li><li class="chapter-item expanded "><a href="getting-started/quick-start.html"><strong aria-hidden="true">1.</strong> Quick Start</a></li><li class="chapter-item expanded "><a href="getting-started/first-handler.html"><strong aria-hidden="true">2.</strong> Your First Handler</a></li><li class="chapter-item expanded "><a href="getting-started/lifecycle.html"><strong aria-hidden="true">3.</strong> Handler Lifecycle</a></li><li class="chapter-item expanded affix "><li class="part-title">SDK Reference</li><li class="chapter-item expanded "><a href="sdk/handlers.html"><strong aria-hidden="true">4.</strong> Handler Macros</a></li><li class="chapter-item expanded "><a href="sdk/request.html"><strong aria-hidden="true">5.</strong> Request</a></li><li class="chapter-item expanded "><a href="sdk/response.html"><strong aria-hidden="true">6.</strong> Response</a></li><li class="chapter-item expanded "><a href="sdk/errors.html"><strong aria-hidden="true">7.</strong> Error Handling</a></li><li class="chapter-item expanded "><a href="sdk/services.html"><strong aria-hidden="true">8.</strong> Services</a><a class="toggle"><div>❱</div></a></li><li><ol class="section"><li class="chapter-item "><a href="sdk/services/storage.html"><strong aria-hidden="true">8.1.</strong> Storage Abstraction</a></li><li class="chapter-item "><a href="sdk/services/database.html"><strong aria-hidden="true">8.2.</strong> Database</a></li><li class="chapter-item "><a href="sdk/services/redis.html"><strong aria-hidden="true">8.3.</strong> Redis</a></li><li class="chapter-item "><a href="sdk/services/ftp.html"><strong aria-hidden="true">8.4.</strong> FTP/SFTP</a></li><li class="chapter-item "><a href="sdk/services/email.html"><strong aria-hidden="true">8.5.</strong> Email</a></li></ol></li><li class="chapter-item expanded "><li class="part-title">Examples</li><li class="chapter-item expanded "><a href="examples/hello-world.html"><strong aria-hidden="true">9.</strong> Hello World</a></li><li class="chapter-item expanded "><a href="examples/json-api.html"><strong aria-hidden="true">10.</strong> JSON API</a></li><li class="chapter-item expanded "><a href="examples/path-params.html"><strong aria-hidden="true">11.</strong> Path Parameters</a></li><li class="chapter-item expanded "><a href="examples/query-params.html"><strong aria-hidden="true">12.</strong> Query Parameters</a></li><li class="chapter-item expanded "><a href="examples/error-handling.html"><strong aria-hidden="true">13.</strong> Error Handling</a></li><li class="chapter-item expanded "><a href="examples/petstore.html"><strong aria-hidden="true">14.</strong> Pet Store Demo</a></li><li class="chapter-item expanded affix "><li class="part-title">API Reference</li><li class="chapter-item expanded "><a href="api/management.html"><strong aria-hidden="true">15.</strong> Management API</a></li><li class="chapter-item expanded "><a href="api/domains.html"><strong aria-hidden="true">16.</strong> Domains</a></li><li class="chapter-item expanded "><a href="api/collections.html"><strong aria-hidden="true">17.</strong> Collections</a></li><li class="chapter-item expanded "><a href="api/services.html"><strong aria-hidden="true">18.</strong> Services</a></li><li class="chapter-item expanded "><a href="api/endpoints.html"><strong aria-hidden="true">19.</strong> Endpoints</a></li></ol>';
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
