    // ===== Global modal/nav scroll-lock =====
    // CSS :has(.modal-visible) lo hace automáticamente en browsers modernos.
    // Este JS es backup para browsers viejos y para asegurar comportamiento.
    // También bloquea el scroll del body cuando el drawer de cuenta o un modal
    // está abierto: sin esto, al hacer scroll DENTRO del drawer el gesto se lo
    // llevaba la página de detrás.
    (function() {
        'use strict';
        if (window.self !== window.top && location.search.indexOf('embed=1') !== -1) {
            document.body.classList.add('is-embedded');
        }
        function anythingOpen() {
            return document.querySelector('.modal-overlay.modal-visible')
                || document.querySelector('.aethravox-account-backdrop.is-visible');
        }
        const apply = () => {
            // Bloqueamos AMBOS: body y documentElement (html). El scroll del
            // viewport en standards mode vive en el <html>; bloquear solo el
            // body no basta (por eso seguía moviéndose el fondo). Con los dos
            // locked, el input real (touch/wheel) ya no desplaza la página.
            const lock = anythingOpen();
            document.documentElement.style.overflow = lock ? 'hidden' : '';
            document.body.style.overflow = lock ? 'hidden' : '';
        };
        const observer = new MutationObserver(apply);
        observer.observe(document.body, {
            attributes: true,
            attributeFilter: ['class'],
            subtree: true,
        });
        // Aplicar al cargar por si el backdrop ya estaba visible (ej. navegación back-forward).
        apply();
    })();

    // ===== Drawer de cuenta (hamburguesa del header, móvil + app-fullscreen) =====
    // Lo abre la hamburguesa del header (.aethravox-header-toggle, id
    // "aethravox-fab-menu" — id conservado del FAB original para no tocar este
    // selector). Drawer fijo a la izquierda con avatar + nombre + rol + links
    // rápidos + logout. Cierre: backdrop, Esc, o click en un link.
    (function() {
        'use strict';
        var fab = document.getElementById('aethravox-fab-menu');
        var drawer = document.getElementById('aethravox-account-drawer');
        var backdrop = document.getElementById('aethravox-account-backdrop');
        var closeBtn = document.getElementById('aethravox-account-close');
        if (!fab || !drawer || !backdrop) return;

        function openDrawer() {
            drawer.hidden = false;
            backdrop.hidden = false;
            // Forzar reflow para que la transición (transform) arranque.
            void drawer.offsetWidth;
            drawer.classList.add('is-open');
            backdrop.classList.add('is-visible');
            fab.setAttribute('aria-expanded', 'true');
        }
        function closeDrawer() {
            drawer.classList.remove('is-open');
            backdrop.classList.remove('is-visible');
            fab.setAttribute('aria-expanded', 'false');
            // Esperar a que termine la transición antes de ocultar (display:none
            // corta la animación).
            setTimeout(function() {
                drawer.hidden = true;
                backdrop.hidden = true;
            }, 200);
        }
        window.aethravoxCloseAccountDrawer = closeDrawer;

        // Restaurar estado del sidebar (PC) antes del primer paint para evitar
        // parpadeo. En móvil/fullscreen el sidebar está oculto y no aplica.
        var SIDEBAR_KEY = 'aethravox_sidebar_collapsed';
        var sidebarEl = document.querySelector('.aethravox-sidebar');
        if (sidebarEl) {
            try {
                if (localStorage.getItem(SIDEBAR_KEY) === '1') {
                    document.body.classList.add('sidebar-collapsed');
                }
            } catch (e) { /* localStorage bloqueado: estado expandido */ }
            if (getComputedStyle(sidebarEl).display !== 'none') {
                fab.setAttribute('aria-expanded',
                    document.body.classList.contains('sidebar-collapsed') ? 'false' : 'true');
            }
        }

        fab.addEventListener('click', function(e) {
            e.stopPropagation();
            // PC con sidebar visible (/hub, /dashboard): contrae/expande el sidebar.
            if (sidebarEl && getComputedStyle(sidebarEl).display !== 'none') {
                var collapsed = document.body.classList.toggle('sidebar-collapsed');
                fab.setAttribute('aria-expanded', collapsed ? 'false' : 'true');
                try { localStorage.setItem(SIDEBAR_KEY, collapsed ? '1' : '0'); } catch (err) {}
                return;
            }
            // Móvil (≤720px, hamburguesa visible): abre el drawer de cuenta
            // (logout/nav). Es el ÚNICO acceso a logout en móvil.
            if (window.matchMedia('(max-width: 720px)').matches) {
                if (drawer.classList.contains('is-open')) closeDrawer();
                else openDrawer();
                return;
            }
            // PC sin sidebar (páginas leaf): flechita ← → vuelve al panel.
            var backUrl = fab.getAttribute('data-back-url') || '/hub';
            if (backUrl && backUrl !== location.pathname) location.href = backUrl;
        });
        backdrop.addEventListener('click', closeDrawer);
        if (closeBtn) closeBtn.addEventListener('click', closeDrawer);
        // Click en un link del drawer → cerrar (comportamiento esperado en móvil).
        drawer.querySelectorAll('.aethravox-account-link').forEach(function(a) {
            a.addEventListener('click', closeDrawer);
        });
        // Esc cierra el drawer (accesibilidad).
        document.addEventListener('keydown', function(e) {
            if (e.key === 'Escape' && drawer.classList.contains('is-open')) {
                closeDrawer();
                fab.focus();
            }
        });
    })();

    // ===== Marca el link activo en el sidebar (PC) y bottom-nav (móvil) =====
    // IIFE separado del toggle de hamburguesa (que retorna temprano si no hay
    // toggle, cosa que ocurre ahora que la navegación es sidebar/bottom-nav).
    (function() {
        'use strict';
        var path = location.pathname;
        // Mapa ruta → data-nav para marcar activo correctamente (p.ej. /docs/5
        // marca "search"; /community/3 marca "explore").
        var map = [
            { nav: 'home',       prefix: '/hub' },
            { nav: 'aris',       prefix: '/aris' },
            { nav: 'explore',    prefix: '/explorar' },
            { nav: 'explore',    prefix: '/communities' },
            { nav: 'explore',    prefix: '/community' },
            { nav: 'categories', prefix: '/categorias' },
            { nav: 'categories', prefix: '/categoria' },
            { nav: 'search',     prefix: '/buscar' },
            { nav: 'search',     prefix: '/docs' },
            { nav: 'settings',   prefix: '/dashboard' },
            { nav: 'aethramail', prefix: '/correo' },
            { nav: 'aethramail', prefix: '/cuenta/correo' },
            { nav: 'theme',      prefix: '/tema' },
            { nav: 'profile',    prefix: '/perfil' },
        ];
        var activeNav = null;
        // Coincidencia exacta primero (p.ej. /hub exacto).
        for (var i = 0; i < map.length; i++) {
            if (path === map[i].prefix) { activeNav = map[i].nav; break; }
        }
        // Si no, la coincidencia de prefijo más larga (p.ej. /docs/5 → search).
        if (!activeNav) {
            var bestLen = 0;
            for (var j = 0; j < map.length; j++) {
                var p = map[j].prefix;
                if (path.startsWith(p + '/') && p.length > bestLen) {
                    bestLen = p.length; activeNav = map[j].nav;
                }
            }
        }
        if (!activeNav) return;
        document.querySelectorAll('[data-nav="' + activeNav + '"]').forEach(function(el) {
            el.classList.add('is-active');
            el.setAttribute('aria-current', 'page');
        });
    })();

    // ===== Sidebar colapsable (PC, >=721px) =====
    // La lógica de colapsar/expander + persistencia en localStorage vive ahora
    // en el handler del botón del header (#aethravox-fab-menu) más arriba:
    // ahí se detecta si el sidebar está visible (PC no-fullscreen) y se
    // alterna body.sidebar-collapsed. Este bloque ya no necesita su propio
    // toggle (el botón interno .aethravox-sidebar-toggle se eliminó del HTML).

    // ===== Foto de perfil + nombre + rol reales (sidebar, bottom-nav y drawer) =====
    // Pide /api/me (devuelve {avatar, username, display, role_css_key,
    // role_display}) y reemplaza el emoji 👤 por la foto real en:
    //   - .side-profile-avatar (sidebar PC)
    //   - .bn-profile-avatar (bottom-nav móvil)
    //   - #aethravox-account-avatar (drawer de cuenta)
    // Y vuelca nombre + rol en el drawer (#aethravox-account-name / -role).
    // NO se cachea en sessionStorage: si el usuario cambia su avatar o su rol
    // custom, se ve reflejado en la próxima navegación (antes quedaba pillado
    // hasta cerrar la pestaña). El fetch es barato y va con credenciales.
    (function() {
        'use strict';
        var sideAvatar = document.querySelector('.side-profile-avatar');
        var bnAvatar = document.querySelector('.bn-profile-avatar');
        var drawerAvatar = document.getElementById('aethravox-account-avatar');
        var hdrAvatar = document.querySelector('.hdr-profile-avatar');
        var hdrAvatarBtn = document.getElementById('aethravox-header-avatar');
        if (!sideAvatar && !bnAvatar && !drawerAvatar && !hdrAvatar) return;

        function paintAvatar(el, url) {
            if (!el || !url) return;
            el.innerHTML = '';
            el.style.backgroundImage = 'url("' + url + '")';
            el.style.backgroundSize = 'cover';
            el.style.backgroundPosition = 'center';
            el.classList.add('has-avatar');
        }

        function apply(data) {
            if (!data || data.is_guest) {
                if (hdrAvatarBtn) hdrAvatarBtn.setAttribute('href', '/login');
                return;
            }
            paintAvatar(sideAvatar, data.avatar);
            paintAvatar(bnAvatar, data.avatar);
            paintAvatar(drawerAvatar, data.avatar);
            paintAvatar(hdrAvatar, data.avatar);
            var display = data.display || data.username || '';
            if (display) {
                var sideLink = document.getElementById('side-profile-link');
                if (sideLink) sideLink.setAttribute('title', display);
                var bnLink = document.getElementById('bn-profile-link');
                if (bnLink) bnLink.setAttribute('title', display);
                if (hdrAvatarBtn) hdrAvatarBtn.setAttribute('title', display);
            }
            var nameEl = document.getElementById('aethravox-account-name');
            if (nameEl && display) nameEl.textContent = display;
            var roleEl = document.getElementById('aethravox-account-role');
            if (roleEl && data.role_display) {
                roleEl.textContent = data.role_display;
                // Clase role-<css_key> para que /role-colors.css pinte el color.
                roleEl.className = 'aethravox-account-role role-badge role-' + (data.role_css_key || 'usuario');
            }
        }

        fetch('/api/me', { credentials: 'same-origin', cache: 'no-store' })
            .then(function(r) { return r.ok ? r.json() : null; })
            .then(apply)
            .catch(function() { /* sin foto: se queda el svg */ });
    })();

    //
    // Tres capas, cada una cubre un caso:
    //  1) Pide permiso de notificaciones una sola vez por navegador, al primer
    //     ingreso en una página autenticada (icono de Ajustes presente).
    //  2) Registra el Service Worker (/sw.js). El SW recibe eventos `push`
    //     del push service del navegador y muestra la notificación incluso si
    //     la pestaña está CERRADA (siempre que el usuario tenga una suscripción
    //     push guardada). Las notificaciones in-page también se enrutan por el
    //     SW (registration.showNotification) para que sobrevivan a la pestaña.
    //  3) Suscribe push con la clave pública VAPID del backend y la POSTea a
    //     /api/notifications/subscribe. Así el backend puede disparar un push
    //     cuando un mensaje/llamada llega a un usuario sin WS activo.
    //
    // El helper window.showDesktopNotification(title, body, opts) lo usan el
    // chat (mensajes) y las llamadas (call_incoming). opts:
    //   { icon, tag, requireInteraction, always, url, conversation_id, type, onclick }
    (function() {
        'use strict';
        var AUTH_PAGE = !!document.querySelector('.aethravox-sidebar');
        window.aethravoxSWReg = null;

        if (!('Notification' in window)) {
            window.showDesktopNotification = function() { return null; };
            window.aethravoxNotifSupported = false;
            window.aethravoxNotifStatus = 'unsupported';
            return;
        }
        window.aethravoxNotifSupported = true;
        window.aethravoxNotifStatus = Notification.permission;

        // Convierte una clave pública VAPID base64url (sin padding) al
        // Uint8Array que espera pushManager.subscribe({applicationServerKey}).
        function urlBase64ToUint8Array(base64url) {
            var padding = '='.repeat((4 - base64url.length % 4) % 4);
            var b64 = (base64url + padding).replace(/-/g, '+').replace(/_/g, '/');
            var raw = atob(b64);
            var arr = new Uint8Array(raw.length);
            for (var i = 0; i < raw.length; i++) arr[i] = raw.charCodeAt(i);
            return arr;
        }

        // Muestra una notificación. Si hay SW registrado, usa
        // registration.showNotification (SW-owned: sobrevive a la pestaña y
        // el click lo gestiona el SW). Si no, cae a new Notification().
        // opts.url / opts.conversation_id / opts.type viajan en notification.data
        // para que el SW abra la conversación correcta al clickar.
        window.showDesktopNotification = function(title, body, opts) {
            opts = opts || {};
            if (Notification.permission !== 'granted') return null;
            if (document.hasFocus() && !opts.always) return null;
            var notifData = {
                url: opts.url || '/chat',
                conversation_id: opts.conversation_id || null,
                type: opts.type || 'message',
            };
            var baseOpts = {
                body: body || '',
                icon: opts.icon || '/static/img/logo.png',
                tag: opts.tag || 'aethravox',
                requireInteraction: !!opts.requireInteraction,
                silent: !!opts.silent,
                data: notifData,
            };
            // Preferir el SW (notificación SW-owned).
            if (window.aethravoxSWReg) {
                try {
                    window.aethravoxSWReg.showNotification(title, baseOpts).catch(function() {});
                    return 'sw';
                } catch (e) { /* caer abajo */ }
            }
            // Fallback: notificación page-owned.
            try {
                var n = new Notification(title, baseOpts);
                n.onclick = function() {
                    window.focus();
                    n.close();
                    if (typeof opts.onclick === 'function') opts.onclick();
                };
                if (!opts.requireInteraction) {
                    setTimeout(function() { try { n.close(); } catch (e) {} }, 8000);
                }
                return n;
            } catch (e) { return null; }
        };

        // ---- Suscribir push al backend ----
        // Pide la clave pública VAPID, suscribe pushManager, y POSTea la
        // subscription a /api/notifications/subscribe. Idempotente: el
        // backend hace ON CONFLICT(endpoint) DO UPDATE.
        async function subscribePush(reg) {
            if (!reg.pushManager) return;
            // Si ya hay suscripción, re-POSTearla (por si el backend la perdió).
            var existing = await reg.pushManager.getSubscription();
            var sub = existing;
            if (!sub) {
                try {
                    var r = await fetch('/api/notifications/vapid-public-key');
                    if (!r.ok) return;
                    var data = await r.json();
                    if (!data || !data.public_key || !data.supported) return;
                    sub = await reg.pushManager.subscribe({
                        userVisibleOnly: true,
                        applicationServerKey: urlBase64ToUint8Array(data.public_key),
                    });
                } catch (e) {
                    console.warn('push subscribe error:', e);
                    return;
                }
            }
            // Enviar la subscription al backend.
            try {
                await fetch('/api/notifications/subscribe', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        endpoint: sub.endpoint,
                        keys: {
                            p256dh: sub.keys ? sub.keys.p256dh : '',
                            auth: sub.keys ? sub.keys.auth : '',
                        },
                    }),
                });
            } catch (e) { console.warn('push subscribe POST error:', e); }
        }

        // ---- Registrar el Service Worker (solo en páginas autenticadas y HTTPS/localhost) ----
        if (AUTH_PAGE && 'serviceWorker' in navigator) {
            window.addEventListener('load', function() {
                navigator.serviceWorker.register('/sw.js', { scope: '/' })
                    .then(function(reg) {
                        window.aethravoxSWReg = reg;
                        // Forzar update si hay SW nuevo.
                        if (reg.waiting) reg.waiting.postMessage({ type: 'skip-waiting' });
                        // Cuando el SW activo cambia, recargar para que la
                        // nueva versión del SW controle la página.
                        var refreshing = false;
                        navigator.serviceWorker.addEventListener('controllerchange', function() {
                            if (refreshing) return;
                            refreshing = true;
                            window.location.reload();
                        });
                        // Si el permiso de notificaciones ya está concedido,
                        // suscribir push cuanto antes. Si no, se suscribirá
                        // tras pedir (y obtener) permiso abajo.
                        if (Notification.permission === 'granted') {
                            subscribePush(reg);
                        }
                    })
                    .catch(function(e) { console.warn('SW register error:', e); });
            });
        }

        // ---- Pedir permiso (una sola vez) ----
        if (!AUTH_PAGE) return;
        if (localStorage.getItem('aethravox_notif_asked') === '1') {
            // Ya preguntado antes: si el permiso está concedido pero el SW
            // aún no ha suscrito (ej. recarga), asegurarnos de suscribir.
            if (Notification.permission === 'granted' && window.aethravoxSWReg) {
                subscribePush(window.aethravoxSWReg);
            }
            return;
        }
        localStorage.setItem('aethravox_notif_asked', '1');

        // Chrome requiere un user gesture para requestPermission().
        let asked = false;
        function ask() {
            if (asked) return;
            asked = true;
            Notification.requestPermission().then(function(p) {
                window.aethravoxNotifStatus = p;
                if (p === 'granted') {
                    if (typeof window.showToast === 'function') {
                        window.showToast('Notificaciones activadas. Te avisaremos de mensajes y llamadas.', 'success');
                    }
                    // Tras conceder, suscribir push si el SW ya está listo.
                    if (window.aethravoxSWReg) {
                        subscribePush(window.aethravoxSWReg);
                    } else {
                        // El SW aún cargando: reintentar en 2s.
                        setTimeout(function() {
                            if (window.aethravoxSWReg) subscribePush(window.aethravoxSWReg);
                        }, 2000);
                    }
                } else if (p === 'denied' && typeof window.showToast === 'function') {
                    window.showToast('Notificaciones bloqueadas. Puedes activarlas luego desde Ajustes.', 'info');
                }
            }).catch(function() {});
            document.removeEventListener('click', ask, true);
            document.removeEventListener('keydown', ask, true);
            document.removeEventListener('touchstart', ask, true);
        }
        document.addEventListener('click', ask, true);
        document.addEventListener('keydown', ask, true);
        document.addEventListener('touchstart', ask, true);
        setTimeout(function() {
            if (!asked) {
                localStorage.removeItem('aethravox_notif_asked');
                document.removeEventListener('click', ask, true);
                document.removeEventListener('keydown', ask, true);
                document.removeEventListener('touchstart', ask, true);
            }
        }, 90000);
    })();

    // ===== Selector de Modo Claro / Oscuro y Tema Visual Global =====
    (function() {
        'use strict';
        var MODE_KEY = 'aethravox_color_mode';
        var THEME_KEY = 'aethravox_theme';
        var headerBtn = document.getElementById('aethravox-mode-toggle');
        var drawerBtn = document.getElementById('aethravox-drawer-mode-toggle');
        var drawerIco = document.getElementById('aethravox-drawer-mode-ico');
        var drawerLabel = document.getElementById('aethravox-drawer-mode-label');
        var headerThemeSel = document.getElementById('aethravox-header-theme');
        var headerArisBtn = document.getElementById('aethravox-header-aris');

        function currentMode() {
            return document.documentElement.getAttribute('data-mode') === 'light' ? 'light' : 'dark';
        }
        function currentTheme() {
            var t = document.documentElement.getAttribute('data-theme');
            return (t === 'shadow' || t === 'commoncents' || t === 'aethravox') ? t : 'aethravox';
        }
        var MOON_SVG = '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/></svg>';
        var SUN_SVG = '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/></svg>';
        function paintControls(mode) {
            var isLight = mode === 'light';
            if (headerBtn) {
                headerBtn.innerHTML = isLight ? SUN_SVG : MOON_SVG;
                headerBtn.title = isLight ? 'Cambiar a modo oscuro' : 'Cambiar a modo claro';
            }
            if (drawerIco) drawerIco.innerHTML = isLight ? SUN_SVG : MOON_SVG;
            if (drawerLabel) drawerLabel.textContent = isLight ? 'Cambiar a Modo Oscuro' : 'Cambiar a Modo Claro';
            document.querySelectorAll('.sidebar-mode-btn[data-side-mode]').forEach(function(b) {
                b.classList.toggle('is-active', b.getAttribute('data-side-mode') === mode);
            });
            document.getElementById('mode-btn-dark')?.classList.toggle('is-active', !isLight);
            document.getElementById('mode-btn-light')?.classList.toggle('is-active', isLight);
            document.getElementById('dash-mode-dark')?.classList.toggle('is-active', !isLight);
            document.getElementById('dash-mode-light')?.classList.toggle('is-active', isLight);
        }
        window.aethravoxSetColorMode = function(mode) {
            var clean = mode === 'light' ? 'light' : 'dark';
            document.documentElement.setAttribute('data-mode', clean);
            try { localStorage.setItem(MODE_KEY, clean); } catch (e) {}
            paintControls(clean);
        };
        paintControls(currentMode());

        function toggleMode() {
            window.aethravoxSetColorMode(currentMode() === 'light' ? 'dark' : 'light');
        }
        if (headerBtn) headerBtn.addEventListener('click', toggleMode);
        if (drawerBtn) drawerBtn.addEventListener('click', toggleMode);
        document.querySelectorAll('.sidebar-mode-btn[data-side-mode]').forEach(function(b) {
            b.addEventListener('click', function() {
                window.aethravoxSetColorMode(b.getAttribute('data-side-mode'));
            });
        });

        function paintThemeControls(theme) {
            if (headerThemeSel && headerThemeSel.value !== theme) {
                headerThemeSel.value = theme;
            }
            document.querySelectorAll('.sidebar-theme-dot[data-side-theme]').forEach(function(b) {
                b.classList.toggle('is-active', b.getAttribute('data-side-theme') === theme);
            });
            document.querySelectorAll('.theme-card[data-theme]').forEach(function(card) {
                var active = card.getAttribute('data-theme') === theme;
                card.classList.toggle('theme-card-active', active);
                var badge = card.querySelector('.badge-ok');
                if (badge) badge.hidden = !active;
            });
        }

        window.aethravoxApplyTheme = function(theme, persistServer) {
            if (theme !== 'aethravox' && theme !== 'shadow' && theme !== 'commoncents') return;
            document.documentElement.setAttribute('data-theme', theme);
            try { localStorage.setItem(THEME_KEY, theme); } catch (e) {}
            var link = document.querySelector('link[data-theme-css]');
            if (link && !link.href.includes('/themes/' + theme + '.css')) {
                link.href = '/static/css/themes/' + theme + '.css?v=1';
            }
            paintThemeControls(theme);
            if (persistServer) {
                fetch('/api/theme/select', {
                    method: 'POST',
                    credentials: 'same-origin',
                    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
                    body: 'theme=' + encodeURIComponent(theme)
                }).catch(function() {});
            }
        };
        paintThemeControls(currentTheme());

        if (headerThemeSel) {
            headerThemeSel.addEventListener('change', function() {
                window.aethravoxApplyTheme(headerThemeSel.value, true);
            });
        }
        document.querySelectorAll('.sidebar-theme-dot[data-side-theme]').forEach(function(b) {
            b.addEventListener('click', function() {
                window.aethravoxApplyTheme(b.getAttribute('data-side-theme'), true);
            });
        });
        if (headerArisBtn) {
            headerArisBtn.addEventListener('click', function() {
                var fab = document.getElementById('aris-quick-fab');
                if (fab && getComputedStyle(fab).display !== 'none') {
                    fab.click();
                } else {
                    window.location.href = '/aris';
                }
            });
        }
    })();

    // ===== Asistente Rápido Contextual de Aris (Drawer flotante global) =====
    (function() {
        'use strict';
        var fab = document.getElementById('aris-quick-fab');
        var drawer = document.getElementById('aris-quick-drawer');
        var backdrop = document.getElementById('aris-quick-backdrop');
        var closeBtn = document.getElementById('aris-quick-close');
        var form = document.getElementById('aris-quick-form');
        var input = document.getElementById('aris-quick-input');
        var msgsEl = document.getElementById('aris-quick-messages');
        var chipsEl = document.getElementById('aris-quick-chips');
        var ctxEl = document.getElementById('aris-quick-context');
        if (!fab || !drawer || !backdrop) return;

        // Ocultar en /aris y en páginas fullscreen para no duplicar controles
        if (location.pathname.startsWith('/aris') || document.body.classList.contains('app-fullscreen')) {
            fab.style.display = 'none';
            return;
        }

        var quickChatId = null;
        var quickSending = false;
        var quickCoreStatus = null;

        function contextLabel() {
            var p = location.pathname;
            if (p.startsWith('/hub')) return 'Panel principal (Hub)';
            if (p.startsWith('/aextube')) return 'aexTube';
            if (p.startsWith('/aexos')) return 'aexOS';
            if (p.startsWith('/playground')) return 'Playground';
            if (p.startsWith('/docs')) return 'Documentos';
            if (p.startsWith('/communities') || p.startsWith('/community')) return 'Comunidades';
            if (p.startsWith('/admin')) return 'Administración';
            if (p.startsWith('/tema')) return 'Temas visuales';
            if (p.startsWith('/buscar')) return 'Búsqueda';
            if (p.startsWith('/perfil')) return 'Perfil';
            return 'Aethravox (' + p + ')';
        }

        function contextSuggestions() {
            var p = location.pathname;
            var hasAnalytics = !!(quickCoreStatus && quickCoreStatus.lanes && quickCoreStatus.lanes.analytics);
            if (p.startsWith('/admin')) {
                return hasAnalytics
                    ? ['Resumen de webs más usadas en 7 días', 'Resume cómo funciona la rotación de APIs']
                    : ['Resume cómo funciona la rotación de APIs', 'Consejos para gestionar roles y permisos'];
            }
            if (p.startsWith('/docs')) {
                return ['Ayúdame a estructurar un documento técnico', 'Resume buenas prácticas de documentación'];
            }
            if (p.startsWith('/aextube')) {
                return ['Dame ideas de títulos atractivos para un vídeo', 'Guión breve de 30s para un Short'];
            }
            if (hasAnalytics) {
                return ['Resumen de webs más usadas en 7 días', '¿Cuántas personas entraron a playground hoy?'];
            }
            return ['¿Qué herramientas hay en Aethravox?', 'Ayúdame a redactar una idea rápida'];
        }

        function esc(s) {
            var d = document.createElement('div');
            d.textContent = s;
            return d.innerHTML;
        }

        function renderChips() {
            if (!chipsEl) return;
            chipsEl.innerHTML = '';
            if (quickCoreStatus && quickCoreStatus.lanes && !quickCoreStatus.lanes.quick_drawer) {
                return;
            }
            contextSuggestions().forEach(function(text) {
                var b = document.createElement('button');
                b.type = 'button';
                b.className = 'aris-quick-chip';
                b.textContent = text;
                b.addEventListener('click', function() {
                    if (input) {
                        input.value = text;
                        submitPrompt(text);
                    }
                });
                chipsEl.appendChild(b);
            });
        }

        function applyQuickLaneAccess(data) {
            quickCoreStatus = data;
            var canQuick = !!(data && data.lanes && data.lanes.quick_drawer);
            if (ctxEl && data && data.role_display) {
                ctxEl.textContent = 'Contexto: ' + contextLabel() + ' · ' + data.role_display;
            }
            if (form && input) {
                input.disabled = !canQuick;
                var sendBtn = document.getElementById('aris-quick-send');
                if (sendBtn) sendBtn.disabled = !canQuick;
                input.placeholder = canQuick ? 'Escribe a Aris...' : 'Carril rápido bloqueado para tu rol';
            }
            if (!canQuick && msgsEl) {
                var welcome = msgsEl.querySelector('.aris-quick-welcome');
                if (welcome) {
                    welcome.innerHTML = (data && data.is_guest)
                        ? 'Inicia sesión o activa <code>aris.lane.quick_drawer</code> para Invitado para usar Aris rápido.'
                        : 'Tu rol (<strong>' + esc((data && data.role_display) || 'Usuario') + '</strong>) no tiene habilitado el Asistente Rápido de Aris (<code>aris.lane.quick_drawer</code>).';
                }
            }
            renderChips();
        }

        async function loadQuickStatus() {
            try {
                var r = await fetch('/api/aris/core/status', { credentials: 'same-origin', cache: 'no-store' });
                if (r.ok) {
                    var data = await r.json();
                    applyQuickLaneAccess(data);
                }
            } catch (e) {}
        }

        function openQuick() {
            if (ctxEl) ctxEl.textContent = 'Contexto: ' + contextLabel();
            renderChips();
            drawer.hidden = false;
            backdrop.hidden = false;
            void drawer.offsetWidth;
            drawer.classList.add('is-open');
            backdrop.classList.add('is-visible');
            loadQuickStatus();
            if (input && !input.disabled) input.focus();
        }

        function closeQuick() {
            drawer.classList.remove('is-open');
            backdrop.classList.remove('is-visible');
            setTimeout(function() {
                drawer.hidden = true;
                backdrop.hidden = true;
            }, 200);
        }

        async function ensureQuickChat() {
            if (quickChatId) return quickChatId;
            try {
                var listRes = await fetch('/api/aris/chats', { credentials: 'same-origin' });
                if (listRes.redirected && listRes.url.indexOf('/login') !== -1) return null;
                if (!listRes.ok) return null;
                var listData = await listRes.json();
                var chats = listData.chats || [];
                var existing = chats.find(function(c) { return c.title === 'Asistente Rápido'; });
                if (existing) {
                    quickChatId = existing.id;
                    return quickChatId;
                }
                var createRes = await fetch('/api/aris/chats', {
                    method: 'POST',
                    credentials: 'same-origin',
                    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
                    body: 'title=' + encodeURIComponent('Asistente Rápido') + '&history_limit=50'
                });
                if (!createRes.ok) return null;
                var created = await createRes.json();
                if (created && created.ok) {
                    quickChatId = created.chat_id;
                    return quickChatId;
                }
            } catch (e) {}
            return null;
        }

        function appendBubble(role, htmlText) {
            var welcome = msgsEl.querySelector('.aris-quick-welcome');
            if (welcome) welcome.remove();
            var div = document.createElement('div');
            div.className = 'aris-quick-msg ' + role;
            div.innerHTML = htmlText;
            msgsEl.appendChild(div);
            msgsEl.scrollTop = msgsEl.scrollHeight;
            return div;
        }

        async function submitPrompt(rawText) {
            if (quickSending) return;
            var text = (rawText || '').trim();
            if (!text) return;
            quickSending = true;
            if (input) input.value = '';
            appendBubble('user', esc(text));
            var botBubble = appendBubble('aris', '<span class="muted">Pensando...</span>');

            var chatId = await ensureQuickChat();
            if (!chatId) {
                botBubble.innerHTML = 'Inicia sesión para conversar con Aris. <a href="/login" style="color:var(--dorado);text-decoration:underline;margin-left:4px;">Iniciar sesión</a>';
                quickSending = false;
                return;
            }

            try {
                var res = await fetch('/api/aris/chats/' + chatId + '/send', {
                    method: 'POST',
                    credentials: 'same-origin',
                    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
                    body: 'source=quick&content=' + encodeURIComponent('[Contexto: ' + contextLabel() + '] ' + text)
                });
                if (!res.ok || !res.body) {
                    botBubble.textContent = 'No se pudo conectar con Aris.';
                    quickSending = false;
                    return;
                }
                var reader = res.body.getReader();
                var decoder = new TextDecoder();
                var buffer = '';
                var full = '';
                var metaBadge = '';
                while (true) {
                    var step = await reader.read();
                    if (step.done) break;
                    buffer += decoder.decode(step.value, { stream: true });
                    var events = buffer.split('\n\n');
                    buffer = events.pop() || '';
                    for (var i = 0; i < events.length; i++) {
                        var lines = events[i].split('\n');
                        var evtName = 'message';
                        var dataParts = [];
                        for (var k = 0; k < lines.length; k++) {
                            if (lines[k].indexOf('event:') === 0) evtName = lines[k].slice(6).trim();
                            else if (lines[k].indexOf('data:') === 0) dataParts.push(lines[k].slice(5).replace(/^ /, ''));
                        }
                        var data = dataParts.join('\n');
                        if (evtName === 'chunk') {
                            var txt = data;
                            try {
                                var p = JSON.parse(data);
                                if (typeof p.text === 'string') txt = p.text;
                            } catch (e) {}
                            full += txt;
                            botBubble.innerHTML = esc(full).replace(/\n/g, '<br>');
                            msgsEl.scrollTop = msgsEl.scrollHeight;
                        } else if (evtName === 'error') {
                            botBubble.innerHTML = '<span style="color:var(--rojo);">' + esc(data) + '</span>';
                        } else if (evtName === 'done') {
                            try {
                                var info = JSON.parse(data);
                                if (typeof info.final_text === 'string' && info.final_text.trim()) {
                                    full = info.final_text;
                                }
                                if (info.provider || info.model) {
                                    metaBadge = '<div class="aris-quick-meta">' + esc((info.provider || '') + (info.model ? ' · ' + info.model : '')) + '</div>';
                                }
                            } catch (e) {}
                        }
                    }
                }
                if (full) {
                    botBubble.innerHTML = esc(full).replace(/\n/g, '<br>') + metaBadge;
                }
            } catch (e) {
                botBubble.textContent = 'Error de conexión con Aris.';
            }
            quickSending = false;
        }

        fab.addEventListener('click', openQuick);
        backdrop.addEventListener('click', closeQuick);
        if (closeBtn) closeBtn.addEventListener('click', closeQuick);
        if (form) {
            form.addEventListener('submit', function(e) {
                e.preventDefault();
                submitPrompt(input ? input.value : '');
            });
        }
        document.addEventListener('keydown', function(e) {
            if (e.key === 'Escape' && drawer.classList.contains('is-open')) closeQuick();
        });
    })();
