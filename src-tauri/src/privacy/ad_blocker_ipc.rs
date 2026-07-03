// ── Cross-platform ad blocking via IPC (non-Windows fallback) ──────────
//
// On Windows, the adblock engine intercepts requests natively via
// WebView2's `WebResourceRequested` event.  On every other platform we
// inject a JavaScript proxy that overrides `fetch` / `XMLHttpRequest`
// and calls back to this module's `check_url` command.
//
// The script also injects cosmetic CSS hiding rules for known ad
// selectors, and periodically syncs the blocked-count back to the
// ShieldState via a second command (`report_blocked`).

use adblock::request::Request;
use adblock::Engine;

use crate::privacy::ad_blocker::AdBlocker;

/// Check a single URL against the adblock engine.
///
/// Called from injected JS on every `fetch` / `XHR` before the request
/// leaves the webview.
pub fn should_block(
    blocker: &AdBlocker,
    url: &str,
    source_url: &str,
    request_type: &str,
) -> bool {
    let Ok(request) = Request::new(url, source_url, request_type) else {
        return false;
    };
    blocker.should_block_request(&request)
}

/// Return the injection script that gets `eval()`'d into every new
/// webview on non-Windows platforms.
pub fn injection_script() -> &'static str {
    ADBLOCK_INJECTION_JS
}

// ═══════════════════════════════════════════════════════════════════════
//  JavaScript injection — ad blocking proxy for non-Windows platforms
// ═══════════════════════════════════════════════════════════════════════

const ADBLOCK_INJECTION_JS: &str = r##"
(function(){
    if (window.__voidBrowserAdBlockInstalled) return;
    window.__voidBrowserAdBlockInstalled = true;

    const invoke = window.__TAURI_INTERNALS__.invoke;
    if (!invoke) return;

    // ── Helpers ──────────────────────────────────────────────────
    const blockedSet = new Set();
    const resourceTypeMap = {
        'fetch':       'xmlhttprequest',
        'xmlhttprequest': 'xmlhttprequest',
        'script':      'script',
        'img':         'image',
        'image':       'image',
        'iframe':      'subdocument',
        'stylesheet':  'stylesheet',
        'font':        'font',
        'media':       'media',
        'websocket':   'websocket',
        'other':       'other',
    };

    function getType(element) {
        if (element.tagName === 'SCRIPT') return 'script';
        if (element.tagName === 'IMG')    return 'image';
        if (element.tagName === 'IFRAME') return 'subdocument';
        if (element.tagName === 'LINK' && element.rel === 'stylesheet') return 'stylesheet';
        if (element.tagName === 'SOURCE' || element.tagName === 'VIDEO' || element.tagName === 'AUDIO') return 'media';
        return 'other';
    }

    let blockedCount = 0;
    function reportBlocked() {
        blockedCount++;
        // Throttled — invoke at most once per 500ms
        if (!window.__blockedBatchTimer) {
            window.__blockedBatchTimer = setTimeout(() => {
                window.__blockedBatchTimer = null;
                if (blockedCount > 0) {
                    try { invoke('report_blocked_count', { count: blockedCount }); } catch(e) {}
                    blockedCount = 0;
                }
            }, 500);
        }
    }

    async function checkUrl(url, type, source) {
        try {
            const blocked = await invoke('check_url', {
                url: url,
                sourceUrl: source || document.baseURI || location.href,
                requestType: type || 'other',
            });
            if (blocked) {
                blockedSet.add(url);
                reportBlocked();
                return true;
            }
            return false;
        } catch(e) {
            return false;
        }
    }

    // ── Override fetch ───────────────────────────────────────────
    const origFetch = window.fetch;
    window.fetch = async function(input, init) {
        const url = (typeof input === 'string') ? input :
                    (input instanceof Request) ? input.url : '';
        if (url) {
            const blocked = await checkUrl(url, 'fetch', location.href);
            if (blocked) return new Response('', { status: 200, statusText: 'Blocked by VoidBrowser' });
        }
        return origFetch.call(this, input, init);
    };

    // ── Override XMLHttpRequest ──────────────────────────────────
    const OrigXHR = window.XMLHttpRequest;
    window.XMLHttpRequest = function() {
        const xhr = new OrigXHR();
        const origOpen = xhr.open.bind(xhr);
        const origSend = xhr.send.bind(xhr);
        let _url = '';

        xhr.open = function(method, url) {
            _url = (typeof url === 'string') ? url : '';
            return origOpen(method, url);
        };

        xhr.send = function(body) {
            if (_url) {
                checkUrl(_url, 'xmlhttprequest', location.href).then(blocked => {
                    if (blocked) {
                        Object.defineProperty(xhr, 'readyState', { get: () => 4 });
                        Object.defineProperty(xhr, 'status', { get: () => 0 });
                        Object.defineProperty(xhr, 'responseText', { get: () => '' });
                        xhr.dispatchEvent(new Event('loadend'));
                        return;
                    }
                });
            }
            return origSend(body);
        };

        return xhr;
    };
    window.XMLHttpRequest.prototype = OrigXHR.prototype;

    // ── Element creation hook (scripts, images, iframes) ─────────
    const origCreateElement = document.createElement.bind(document);
    document.createElement = function(tagName, options) {
        const el = origCreateElement(tagName, options);
        if (tagName.toLowerCase() === 'script' || tagName.toLowerCase() === 'img' || tagName.toLowerCase() === 'iframe') {
            const origSetAttr = el.setAttribute.bind(el);
            el.setAttribute = function(name, value) {
                if ((name === 'src' || name === 'href') && typeof value === 'string') {
                    const type = getType(el);
                    checkUrl(value, type, location.href).then(blocked => {
                        if (blocked && el.parentNode) {
                            el.parentNode.removeChild(el);
                        }
                    });
                }
                return origSetAttr(name, value);
            };
            // Also intercept direct .src assignment
            if (tagName.toLowerCase() !== 'iframe') {
                try {
                    Object.defineProperty(el, 'src', {
                        set: function(value) {
                            const type = getType(el);
                            checkUrl(value, type, location.href).then(blocked => {
                                if (!blocked) {
                                    el.setAttribute('src', value);
                                }
                            });
                        }
                    });
                } catch(e) { /* readonly in some contexts */ }
            }
        }
        return el;
    };

    // ── MutationObserver for dynamically injected elements ───────
    const observer = new MutationObserver(mutations => {
        for (const mutation of mutations) {
            for (const node of mutation.addedNodes) {
                if (node.nodeType !== 1) continue;
                const el = node;
                const tag = el.tagName ? el.tagName.toLowerCase() : '';
                if ((tag === 'script' || tag === 'img' || tag === 'iframe') && el.src) {
                    checkUrl(el.src, getType(el), location.href).then(blocked => {
                        if (blocked && el.parentNode) el.parentNode.removeChild(el);
                    });
                }
                // Check descendants
                if (el.querySelectorAll) {
                    el.querySelectorAll('script[src], img[src], iframe[src]').forEach(child => {
                        const src = child.src || child.getAttribute('src') || '';
                        if (src && !blockedSet.has(src)) {
                            checkUrl(src, getType(child), location.href).then(blocked => {
                                if (blocked && child.parentNode) child.parentNode.removeChild(child);
                            });
                        }
                    });
                }
            }
        }
    });
    observer.observe(document.documentElement, { childList: true, subtree: true });
})();
"##;