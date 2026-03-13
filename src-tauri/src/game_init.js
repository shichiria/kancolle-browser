
(function() {
    // --- CSS applied to ALL frames (including cross-origin game iframes) ---
    // This removes scrollbars everywhere in the WebView2 window.
    var COMMON_CSS = `
        html, body {
            margin: 0 !important;
            padding: 0 !important;
            overflow: hidden !important;
        }
        * {
            scrollbar-width: none !important;
            -ms-overflow-style: none !important;
        }
        *::-webkit-scrollbar { display: none !important; }
    `;

    // --- CSS applied only to the top-level DMM frame ---
    var TOP_CSS = `
        html, body {
            background-color: black !important;
            width: 100% !important;
            height: 100% !important;
        }
        .dmm-ntgnavi, .area-naviapp, #ntg-recommend,
        #foot, #foot+img,
        .gamesResetStyle > header,
        .gamesResetStyle > footer,
        .gamesResetStyle > aside,
        #page header, #page footer, .nav_area,
        .area-biling, .peri-header, .peri-footer {
            display: none !important;
        }
        #w, #main-ntg, #page {
            margin: 0 !important;
            padding: 0 !important;
            width: 100% !important;
            background: none !important;
            overflow: hidden !important;
        }
        #main-ntg {
            margin: 0 !important;
            position: static !important;
        }
        #area-game {
            margin: 0 !important;
            padding: 0 !important;
            width: 1200px !important;
            height: 720px !important;
            position: relative !important;
            overflow: hidden !important;
        }
        #game_frame {
            position: fixed !important;
            top: 28px !important;
            left: 0 !important;
            z-index: 10000 !important;
            width: 1200px !important;
            height: 720px !important;
            border: none !important;
            overflow: hidden !important;
        }
        /* Control bar */
        #kc-control-bar {
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            height: 28px;
            z-index: 99999;
            background: #16213e;
            display: flex;
            align-items: center;
            padding: 0 8px;
            gap: 8px;
            font-family: -apple-system, BlinkMacSystemFont, sans-serif;
            font-size: 11px;
            color: #e0e0e0;
            border-bottom: 1px solid #0f3460;
            user-select: none;
            -webkit-user-select: none;
        }
        #kc-control-bar select {
            font-size: 11px;
            padding: 1px 4px;
            background: #0f3460;
            color: #e0e0e0;
            border: 1px solid #1a4080;
            border-radius: 3px;
            outline: none;
            cursor: pointer;
        }
        #kc-control-bar select:hover { background: #1a4080; }
        #kc-control-bar button {
            font-size: 12px;
            padding: 1px 8px;
            background: #0f3460;
            color: #e0e0e0;
            border: 1px solid #1a4080;
            border-radius: 3px;
            cursor: pointer;
            line-height: 1.4;
        }
        #kc-control-bar button:hover { background: #1a4080; }
        #kc-control-bar button.muted {
            background: rgba(233,69,96,0.2);
            border-color: rgba(233,69,96,0.4);
        }
        #kc-control-bar .spacer { flex: 1; }
        #kc-control-bar .label { font-size: 10px; color: #666; }
    `;

    var isTop = false;
    try { isTop = (window.self === window.top); } catch(e) {}

    var cssText = isTop ? (COMMON_CSS + TOP_CSS) : COMMON_CSS;

    // Inject style — use MutationObserver on document for WebView2 compatibility
    function injectStyle() {
        if (document.getElementById('kc-browser-style')) return true;
        var target = document.head || document.documentElement;
        if (!target) return false;
        var style = document.createElement('style');
        style.id = 'kc-browser-style';
        style.textContent = cssText;
        target.appendChild(style);
        return true;
    }

    if (!injectStyle()) {
        var obs = new MutationObserver(function(mutations, observer) {
            if (injectStyle()) observer.disconnect();
        });
        obs.observe(document, { childList: true, subtree: true });
    }
    document.addEventListener('DOMContentLoaded', function() { injectStyle(); });

    // Control bar — top frame only
    if (!isTop) return;

    function addControlBar() {
        if (document.getElementById('kc-control-bar')) return;
        var parent = document.body || document.documentElement;
        if (!parent) return;
        var bar = document.createElement('div');
        bar.id = 'kc-control-bar';
        bar.innerHTML = '<select id="kc-zoom">'
            + '<option value="0.5">50%</option>'
            + '<option value="0.67">67%</option>'
            + '<option value="0.75">75%</option>'
            + '<option value="1">100%</option>'
            + '<option value="1.25">125%</option>'
            + '<option value="1.5">150%</option>'
            + '<option value="2">200%</option>'
            + '</select>'
            + '<button id="kc-mute">\u{1f50a}</button>'
            + '<button id="kc-formation" title="\u{9663}\u{5F62}\u{8A18}\u{61B6}">\u{9663}\u{5F62}</button>'
            + '<button id="kc-taiha" title="\u{5927}\u{7834}\u{8B66}\u{544A}">\u{26A0}\u{5927}\u{7834}</button>'
            + '<button id="kc-minimap" title="\u{30DF}\u{30CB}\u{30DE}\u{30C3}\u{30D7}">MAP</button>'
            + '<span class="spacer"></span>'
            + '<span class="label">KanColle Browser</span>';
        parent.appendChild(bar);

        // Restore saved zoom
        var saved = localStorage.getItem('kc-game-zoom');
        if (saved) {
            document.getElementById('kc-zoom').value = saved;
            var z = parseFloat(saved);
            if (z && z !== 1 && window.__TAURI_INTERNALS__) {
                window.__TAURI_INTERNALS__.invoke('set_game_zoom', { zoom: z });
            }
        }

        document.getElementById('kc-zoom').addEventListener('change', function() {
            var z = parseFloat(this.value);
            localStorage.setItem('kc-game-zoom', String(z));
            window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke('set_game_zoom', { zoom: z });
        });

        // Restore mute state from backend
        var muted = false;
        var muteBtn = document.getElementById('kc-mute');
        if (window.__TAURI_INTERNALS__) {
            window.__TAURI_INTERNALS__.invoke('get_game_mute').then(function(m) {
                muted = !!m;
                muteBtn.textContent = muted ? '\u{1f507}' : '\u{1f50a}';
                muteBtn.className = muted ? 'muted' : '';
                if (muted) {
                    window.__TAURI_INTERNALS__.invoke('toggle_game_mute', { muted: true });
                }
            }).catch(function() {});
        }
        muteBtn.addEventListener('click', function() {
            muted = !muted;
            this.textContent = muted ? '\u{1f507}' : '\u{1f50a}';
            this.className = muted ? 'muted' : '';
            window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke('toggle_game_mute', { muted: muted });
        });

        // Formation hint toggle
        var fmtEnabled = true;
        var fmtBtn = document.getElementById('kc-formation');
        if (window.__TAURI_INTERNALS__) {
            window.__TAURI_INTERNALS__.invoke('get_formation_hint_enabled').then(function(e) {
                fmtEnabled = !!e;
                fmtBtn.className = fmtEnabled ? '' : 'muted';
                fmtBtn.title = fmtEnabled ? '\u{9663}\u{5F62}\u{8A18}\u{61B6} ON' : '\u{9663}\u{5F62}\u{8A18}\u{61B6} OFF';
            }).catch(function() {});
        }
        fmtBtn.addEventListener('click', function() {
            fmtEnabled = !fmtEnabled;
            this.className = fmtEnabled ? '' : 'muted';
            this.title = fmtEnabled ? '\u{9663}\u{5F62}\u{8A18}\u{61B6} ON' : '\u{9663}\u{5F62}\u{8A18}\u{61B6} OFF';
            window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke('set_formation_hint_enabled', { enabled: fmtEnabled });
        });

        // Taiha alert toggle
        var taihaEnabled = true;
        var taihaBtn = document.getElementById('kc-taiha');
        if (window.__TAURI_INTERNALS__) {
            window.__TAURI_INTERNALS__.invoke('get_taiha_alert_enabled').then(function(e) {
                taihaEnabled = !!e;
                taihaBtn.className = taihaEnabled ? '' : 'muted';
                taihaBtn.title = taihaEnabled ? '\u{5927}\u{7834}\u{8B66}\u{544A} ON' : '\u{5927}\u{7834}\u{8B66}\u{544A} OFF';
            }).catch(function() {});
        }
        taihaBtn.addEventListener('click', function() {
            taihaEnabled = !taihaEnabled;
            this.className = taihaEnabled ? '' : 'muted';
            this.title = taihaEnabled ? '\u{5927}\u{7834}\u{8B66}\u{544A} ON' : '\u{5927}\u{7834}\u{8B66}\u{544A} OFF';
            window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke('set_taiha_alert_enabled', { enabled: taihaEnabled });
        });

        // Minimap toggle
        var minimapEnabled = true;
        var minimapBtn = document.getElementById('kc-minimap');
        if (window.__TAURI_INTERNALS__) {
            window.__TAURI_INTERNALS__.invoke('get_minimap_enabled').then(function(e) {
                minimapEnabled = !!e;
                minimapBtn.className = minimapEnabled ? '' : 'muted';
            }).catch(function() {});
        }
        minimapBtn.addEventListener('click', function() {
            if (window.__TAURI_INTERNALS__) {
                window.__TAURI_INTERNALS__.invoke('toggle_minimap').then(function(enabled) {
                    minimapEnabled = enabled;
                    minimapBtn.className = minimapEnabled ? '' : 'muted';
                }).catch(function() {});
            }
        });
    }

    if (document.body) addControlBar();
    else document.addEventListener('DOMContentLoaded', addControlBar);
})();
