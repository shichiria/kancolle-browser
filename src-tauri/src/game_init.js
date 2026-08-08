
(function() {
    // This script is a DMM host-page shim. It never reads or modifies the
    // cross-origin KanColle iframe document.
    var GAME_WIDTH = '__KC_GAME_WIDTH__px';
    var GAME_HEIGHT = '__KC_GAME_HEIGHT__px';
    var CONTROL_BAR_HEIGHT = '__KC_CONTROL_BAR_HEIGHT__px';
    var LAYOUT_DIAGNOSTICS = __KC_LAYOUT_DIAGNOSTICS__;

    // WebView2 runs initialization scripts in every frame. The KanColle gadget is
    // cross-origin and must remain indistinguishable from an unmodified browser
    // page, so only the top-level DMM container may be changed.
    var isTop = false;
    try { isTop = (window.self === window.top); } catch(e) {}
    if (!isTop) return;

    // Spoof navigator.userAgentData to look like Edge (instead of WebView2-flavored brands).
    // DMM appears to inspect Sec-CH-UA / userAgentData.brands and bounce non-Edge browsers
    // back to login. Without this, login succeeds but play.games.dmm.com immediately
    // redirects to /service/login/password.
    try {
        var fakeBrands = [
            { brand: "Microsoft Edge", version: "__KC_EDGE_MAJOR_VERSION__" },
            { brand: "Chromium", version: "__KC_EDGE_MAJOR_VERSION__" },
            { brand: "Not_A Brand", version: "24" }
        ];
        var fakeFullVersionList = [
            { brand: "Microsoft Edge", version: "__KC_EDGE_FULL_VERSION__" },
            { brand: "Chromium", version: "__KC_EDGE_FULL_VERSION__" },
            { brand: "Not_A Brand", version: "24.0.0.0" }
        ];
        var fakeUaData = {
            brands: fakeBrands,
            mobile: false,
            platform: "Windows",
            getHighEntropyValues: function(hints) {
                return Promise.resolve({
                    brands: fakeBrands,
                    mobile: false,
                    platform: "Windows",
                    platformVersion: "15.0.0",
                    architecture: "x86",
                    bitness: "64",
                    model: "",
                    uaFullVersion: "__KC_EDGE_FULL_VERSION__",
                    fullVersionList: fakeFullVersionList
                });
            },
            toJSON: function() {
                return { brands: fakeBrands, mobile: false, platform: "Windows" };
            }
        };
        Object.defineProperty(navigator, 'userAgentData', {
            get: function() { return fakeUaData; },
            configurable: true
        });
    } catch(e) {}

    // --- CSS applied only to the top-level DMM frame ---
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
            width: __KC_GAME_WIDTH__px !important;
            height: __KC_GAME_HEIGHT__px !important;
            position: relative !important;
            overflow: hidden !important;
        }
        #game_frame {
            position: fixed !important;
            top: __KC_CONTROL_BAR_HEIGHT__px !important;
            left: 0 !important;
            z-index: 1 !important;
            width: __KC_GAME_WIDTH__px !important;
            height: __KC_GAME_HEIGHT__px !important;
            border: none !important;
            overflow: hidden !important;
        }
        /* Control bar */
        #kc-control-bar {
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            height: __KC_CONTROL_BAR_HEIGHT__px;
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
        #kc-airbase-supply-warning {
            position: fixed;
            left: 278px;
            top: calc(__KC_CONTROL_BAR_HEIGHT__px + 480px);
            z-index: 99998;
            display: none;
            align-items: center;
            gap: 7px;
            padding: 7px 13px;
            border: 2px solid #ffca45;
            border-radius: 8px;
            background: rgba(92, 35, 10, 0.94);
            color: #fff3b0;
            box-shadow: 0 0 15px rgba(255, 174, 32, 0.9);
            font-family: -apple-system, BlinkMacSystemFont, sans-serif;
            font-size: 17px;
            font-weight: 800;
            line-height: 1;
            pointer-events: none;
            user-select: none;
            animation: kc-airbase-supply-blink 0.85s steps(2, start) infinite;
        }
        #kc-airbase-supply-warning .fuel {
            font-size: 23px;
        }
        #kc-airbase-supply-warning .detail {
            display: block;
            margin-top: 3px;
            color: #ffd56b;
            font-size: 9px;
            font-weight: 600;
        }
        @keyframes kc-airbase-supply-blink {
            0%, 100% { opacity: 1; transform: scale(1); }
            50% { opacity: 0.35; transform: scale(0.97); }
        }
    `;

    // Persist a compact DOM/layout snapshot in the session log. DMM changes
    // its wrapper markup periodically, and this makes a blank/incorrect game
    // surface diagnosable without asking the player to open DevTools.
    function reportLayout(stage) {
        if (!LAYOUT_DIAGNOSTICS || !isTop || !window.__TAURI_INTERNALS__) return;
        try {
            function describe(el) {
                var rect = el.getBoundingClientRect();
                return {
                    tag: el.tagName,
                    id: el.id || '',
                    className: typeof el.className === 'string' ? el.className : '',
                    src: el.getAttribute && (el.getAttribute('src') || ''),
                    rect: [Math.round(rect.x), Math.round(rect.y), Math.round(rect.width), Math.round(rect.height)]
                };
            }
            var frames = Array.from(document.querySelectorAll('iframe')).map(describe);
            var candidates = Array.from(document.querySelectorAll('[id*="game" i], [class*="game" i]'))
                .slice(0, 40).map(describe);
            var children = document.body
                ? Array.from(document.body.children).slice(0, 40).map(describe)
                : [];
            window.__TAURI_INTERNALS__.invoke('log_frontend_event', {
                level: 'info',
                source: 'game-content:layout',
                message: JSON.stringify({
                    stage: stage,
                    url: location.href,
                    viewport: [window.innerWidth, window.innerHeight],
                    frames: frames,
                    candidates: candidates,
                    bodyChildren: children
                })
            });
        } catch(e) {}
    }

    if (isTop && LAYOUT_DIAGNOSTICS) {
        document.addEventListener('DOMContentLoaded', function() { reportLayout('dom-content-loaded'); });
        setTimeout(function() { reportLayout('after-3s'); }, 3000);
        setTimeout(function() { reportLayout('after-10s'); }, 10000);
    }

    // Flatten only the iframe's ancestor stacking contexts. DMM renders payment
    // confirmation as a sibling <dialog> (z-index 100), so siblings must remain
    // visible and the game iframe must stay below that dialog.
    function isolateGameFrame() {
        if (!isTop) return false;
        var frame = document.getElementById('game_frame');
        if (!frame || !document.body) return false;

        var node = frame;
        while (node.parentElement && node.parentElement !== document.body) {
            var parent = node.parentElement;
            parent.style.setProperty('position', 'static', 'important');
            parent.style.setProperty('transform', 'none', 'important');
            parent.style.setProperty('z-index', 'auto', 'important');
            parent.style.setProperty('overflow', 'visible', 'important');
            node = parent;
        }
        frame.style.setProperty('display', 'block', 'important');
        frame.style.setProperty('visibility', 'visible', 'important');
        frame.style.setProperty('opacity', '1', 'important');
        // CSS overflow on an iframe element does not suppress the embedded
        // WebView2 viewport's own scrollbars. The host-side scrolling attribute
        // does, without reading or modifying the cross-origin game document.
        frame.setAttribute('scrolling', 'no');
        frame.style.setProperty('position', 'fixed', 'important');
        frame.style.setProperty('top', CONTROL_BAR_HEIGHT, 'important');
        frame.style.setProperty('left', '0', 'important');
        frame.style.setProperty('width', GAME_WIDTH, 'important');
        frame.style.setProperty('height', GAME_HEIGHT, 'important');
        frame.style.setProperty('z-index', '1', 'important');
        return true;
    }

    if (isTop) {
        var layoutObserver = new MutationObserver(function() { isolateGameFrame(); });
        layoutObserver.observe(document, { childList: true, subtree: true });
        document.addEventListener('DOMContentLoaded', isolateGameFrame);
        var layoutChecks = 0;
        var layoutTimer = setInterval(function() {
            isolateGameFrame();
            layoutChecks += 1;
            if (layoutChecks >= 15) {
                clearInterval(layoutTimer);
                layoutObserver.disconnect();
            }
        }, 2000);
    }

    var cssText = COMMON_CSS + TOP_CSS;

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
            + '<button id="kc-reload" title="ゲームをリロード">\u{21bb} リロード</button>'
            + '<button id="kc-screenshot" title="ゲーム画面をPNG保存">\u{1f4f7} 撮影</button>'
            + '<button id="kc-formation" title="\u{9663}\u{5F62}\u{8A18}\u{61B6}">\u{1F3AF} \u{9663}\u{5F62}</button>'
            + '<button id="kc-taiha" title="\u{5927}\u{7834}\u{8B66}\u{544A}">\u{26A0} \u{5927}\u{7834}</button>'
            + '<button id="kc-minimap" title="\u{30DF}\u{30CB}\u{30DE}\u{30C3}\u{30D7}">\u{1F5FA} MAP</button>'
            + '<button id="kc-battle-info" title="\u{6226}\u{95D8}\u{60C5}\u{5831}">\u{2694} \u{6226}\u{95D8}</button>'
            + '<button id="kc-quest" title="\u{4EFB}\u{52D9}\u{30A6}\u{30A3}\u{30F3}\u{30C9}\u{30A6}">\u{1F4DC} \u{4EFB}\u{52D9}</button>'
            + '<button id="kc-kantai" title="\u{8266}\u{968A}\u{30D1}\u{30CD}\u{30EB}">\u{2693} \u{8266}\u{968A}</button>'
            + '<button id="kc-improve" title="\u{6539}\u{4FEE}\u{30A6}\u{30A3}\u{30F3}\u{30C9}\u{30A6}">\u{1F527} \u{6539}\u{4FEE}</button>'
            + '<button id="kc-ships" title="\u{8266}\u{5A18}\u{30A6}\u{30A3}\u{30F3}\u{30C9}\u{30A6}">\u{1F467} \u{8266}\u{5A18}</button>'
            + '<button id="kc-event" title="\u{30A4}\u{30D9}\u{30F3}\u{30C8}\u{9032}\u{884C}\u{7BA1}\u{7406}">\u{1F3AA} \u{30A4}\u{30D9}\u{30F3}\u{30C8}</button>'
            + '<span class="spacer"></span>'
            + '<span class="label">KanColle Browser</span>'
            + '<button id="kc-mgmt" title="\u{7BA1}\u{7406}\u{30D1}\u{30CD}\u{30EB}">\u{1F4CA} \u{7BA1}\u{7406}</button>';
        parent.appendChild(bar);

        // Event-map LBAS supply warning. This lives in the trusted DMM host
        // frame, above the cross-origin game iframe.
        var airbaseWarning = document.createElement('div');
        airbaseWarning.id = 'kc-airbase-supply-warning';
        airbaseWarning.innerHTML = '<span class="fuel">\u{26FD}</span>'
            + '<span>\u{88DC}\u{7D66}'
            + '<small class="detail">\u{30A4}\u{30D9}\u{30F3}\u{30C8}\u{57FA}\u{5730}\u{822A}\u{7A7A}\u{968A}</small>'
            + '</span>';
        parent.appendChild(airbaseWarning);

        function refreshAirbaseSupplyWarning() {
            if (!window.__TAURI_INTERNALS__) return;
            Promise.all([
                window.__TAURI_INTERNALS__.invoke('get_current_screen'),
                window.__TAURI_INTERNALS__.invoke('get_air_bases')
            ]).then(function(values) {
                var screen = values[0];
                var bases = Array.isArray(values[1]) ? values[1] : [];
                var needsSupply = bases.some(function(base) {
                    // Event areas use IDs 20 and above. Ignore empty squadrons.
                    if (!base || Number(base.area_id) < 20 || !Array.isArray(base.planes)) {
                        return false;
                    }
                    return base.planes.some(function(plane) {
                        if (!plane || Number(plane.slotid) <= 0) return false;
                        return Number(plane.state) === 2
                            || Number(plane.count) < Number(plane.max_count);
                    });
                });
                airbaseWarning.style.display =
                    screen === 'SortieSelectEvent' && needsSupply ? 'flex' : 'none';
            }).catch(function() {
                airbaseWarning.style.display = 'none';
            });
        }
        refreshAirbaseSupplyWarning();
        setInterval(refreshAirbaseSupplyWarning, 500);

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

        // Reload the DMM host page. This recreates the game iframe naturally
        // while preserving the WebView2 session, cookies, and proxy settings.
        document.getElementById('kc-reload').addEventListener('click', function() {
            window.location.reload();
        });

        var screenshotBtn = document.getElementById('kc-screenshot');
        screenshotBtn.addEventListener('click', function() {
            if (!window.__TAURI_INTERNALS__ || screenshotBtn.disabled) return;
            var originalText = screenshotBtn.textContent;
            screenshotBtn.disabled = true;
            screenshotBtn.textContent = '\u{23f3} 保存中';
            window.__TAURI_INTERNALS__.invoke('take_game_screenshot')
                .then(function(path) {
                    screenshotBtn.textContent = '\u{2713} 保存';
                    screenshotBtn.title = '保存先: ' + path;
                })
                .catch(function(error) {
                    screenshotBtn.textContent = '\u{26a0} 失敗';
                    screenshotBtn.title = String(error);
                    console.error('Screenshot failed:', error);
                })
                .finally(function() {
                    setTimeout(function() {
                        screenshotBtn.textContent = originalText;
                        screenshotBtn.disabled = false;
                    }, 1500);
                });
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

        // Battle info toggle
        var battleInfoEnabled = true;
        var battleInfoBtn = document.getElementById('kc-battle-info');
        if (window.__TAURI_INTERNALS__) {
            window.__TAURI_INTERNALS__.invoke('get_battle_info_enabled').then(function(e) {
                battleInfoEnabled = !!e;
                battleInfoBtn.className = battleInfoEnabled ? '' : 'muted';
                battleInfoBtn.title = battleInfoEnabled ? '\u{6226}\u{95D8}\u{60C5}\u{5831} ON' : '\u{6226}\u{95D8}\u{60C5}\u{5831} OFF';
            }).catch(function() {});
        }
        battleInfoBtn.addEventListener('click', function() {
            battleInfoEnabled = !battleInfoEnabled;
            this.className = battleInfoEnabled ? '' : 'muted';
            this.title = battleInfoEnabled ? '\u{6226}\u{95D8}\u{60C5}\u{5831} ON' : '\u{6226}\u{95D8}\u{60C5}\u{5831} OFF';
            window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke('set_battle_info_enabled', { enabled: battleInfoEnabled });
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

        // Management panel toggle (show/hide React SPA window)
        var mgmtBtn = document.getElementById('kc-mgmt');
        mgmtBtn.addEventListener('click', function() {
            window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke('toggle_management_window');
        });

        // Kantai (fleet) panel toggle
        var kantaiBtn = document.getElementById('kc-kantai');
        kantaiBtn.addEventListener('click', function() {
            window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke('toggle_kantai_window');
        });

        // Quest window toggle
        var questBtn = document.getElementById('kc-quest');
        questBtn.addEventListener('click', function() {
            window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke('toggle_quests_window');
        });

        // Improvement (\u{6539}\u{4FEE}) window toggle
        var improveBtn = document.getElementById('kc-improve');
        improveBtn.addEventListener('click', function() {
            window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke('toggle_improvement_window');
        });

        // Ship list (\u{8266}\u{5A18}) window toggle
        var shipsBtn = document.getElementById('kc-ships');
        shipsBtn.addEventListener('click', function() {
            window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke('toggle_ships_window');
        });

        // Event progress window toggle
        var eventBtn = document.getElementById('kc-event');
        eventBtn.addEventListener('click', function() {
            window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke('toggle_event_window');
        });
    }

    if (document.body) addControlBar();
    else document.addEventListener('DOMContentLoaded', addControlBar);
})();
