/* ============================================================
   Aethravox Studios — i18n (internacionalización) 100% cliente
   ------------------------------------------------------------
   Sin backend, sin Python, sin frameworks. Solo JS + JSON.

   - Idioma por defecto FIRME: español (es). Es el texto original
     del HTML, así que aunque este script no cargue, se ve español.
   - El idioma SOLO cambia si el usuario lo elige explícitamente en
     Ajustes → Idioma (dashboard). No hay autodetección del navegador
     (evita que la web aparezca en inglés sin que el usuario lo pida).
   - El texto original en español de cada elemento se captura la
     primera vez; al volver a español se RESTAURA ese original
     (no se cae al inglés).
   - Fallback: idioma actual → inglés → español original → clave.
   - Diccionarios en /static/i18n/lang/<lang>.json, cachéados en
     localStorage para evitar parpadeo en visitas repetidas.

   Atributos soportados:
     data-i18n="clave"              → textContent
     data-i18n-html="clave"         → innerHTML (texto con markup)
     data-i18n-placeholder="clave"  → placeholder
     data-i18n-title="clave"        → title (tooltip)
     data-i18n-aria-label="clave"   → aria-label
     data-i18n-alt="clave"          → alt

   API pública:
     i18n.t(clave, fallback?)       → traduce una cadena suelta
     i18n.setLang('en')             → cambia de idioma y re-traduce
     i18n.getLang()                 → idioma actual
     i18n.supported                 → ['es','en','fr','de','pt','it']
     i18n.apply(root?)              → (re)aplica traducciones al DOM
   Evento:
     document.addEventListener('languagechange', e => e.detail.lang)
   ============================================================ */
(function () {
    'use strict';

    var SUPPORTED = ['es', 'en', 'fr', 'de', 'pt', 'it'];
    var DEFAULT_LANG = 'es';          // español = texto original del HTML
    var FALLBACK_LANG = 'en';         // segundo nivel de fallback
    var STORAGE_KEY = 'aethravox_lang';
    var CACHE_PREFIX = 'aethravox_i18n_v2_';

    var dict = {};        // { lang: { clave: texto } }
    var current = DEFAULT_LANG;

    function isSupported(l) { return SUPPORTED.indexOf(l) >= 0; }

    // Idioma inicial: solo el guardado por el usuario; si no, español.
    // NO se usa navigator.language: la web es española por defecto y solo
    // cambia cuando el usuario lo pide.
    function detect() {
        try {
            var saved = localStorage.getItem(STORAGE_KEY);
            if (saved && isSupported(saved)) return saved;
        } catch (e) {}
        return DEFAULT_LANG;
    }

    // Carga un diccionario (fetch + cache en localStorage).
    function loadLang(lang) {
        if (!isSupported(lang)) return Promise.resolve();
        if (dict[lang]) return Promise.resolve();
        try {
            var cached = localStorage.getItem(CACHE_PREFIX + lang);
            if (cached) { dict[lang] = JSON.parse(cached); return Promise.resolve(); }
        } catch (e) {}
        return fetch('/static/i18n/lang/' + lang + '.json', { cache: 'force-cache' })
            .then(function (r) { if (!r.ok) throw new Error('HTTP ' + r.status); return r.json(); })
            .then(function (d) {
                dict[lang] = d || {};
                try { localStorage.setItem(CACHE_PREFIX + lang, JSON.stringify(d)); } catch (e) {}
            })
            .catch(function (e) { console.warn('[i18n] no se pudo cargar "' + lang + '.json":', e); });
    }

    // Traduce una clave con fallback en cascada.
    // IMPORTANTE: cuando el idioma actual es el español (DEFAULT_LANG), NO se
    // consultan los diccionarios: se devuelve el fallback (el texto original
    // del HTML), que es el español de origen. Así al volver a español se
    // restaura el texto original y no se queda en inglés.
    function t(key, fallback) {
        if (current !== DEFAULT_LANG) {
            if (dict[current] && dict[current][key] != null) return dict[current][key];
            if (current !== FALLBACK_LANG && dict[FALLBACK_LANG] && dict[FALLBACK_LANG][key] != null) return dict[FALLBACK_LANG][key];
        }
        return fallback != null ? fallback : key;
    }

    // Aplica las traducciones al DOM (o a un subárbol).
    function apply(root) {
        root = root || document;

        var ATTR_MAP = [
            ['data-i18n', null],                // textContent
            ['data-i18n-html', 'innerHTML'],
            ['data-i18n-placeholder', 'placeholder'],
            ['data-i18n-title', 'title'],
            ['data-i18n-aria-label', 'aria-label'],
            ['data-i18n-alt', 'alt']
        ];
        for (var m = 0; m < ATTR_MAP.length; m++) {
            var attr = ATTR_MAP[m][0];
            var target = ATTR_MAP[m][1];
            var list = root.querySelectorAll('[' + attr + ']');
            for (var i = 0; i < list.length; i++) {
                var el = list[i];
                var key = el.getAttribute(attr);
                // Capturar el valor ORIGINAL (español del HTML) la primera
                // vez que se toca el elemento, para poder restaurarlo al
                // volver a español. Se guarda por atributo.
                if (!el._i18nOrig) el._i18nOrig = {};
                if (!(attr in el._i18nOrig)) {
                    if (target === null) el._i18nOrig[attr] = el.textContent;
                    else if (target === 'innerHTML') el._i18nOrig[attr] = el.innerHTML;
                    else el._i18nOrig[attr] = el.getAttribute(target);
                }
                var val = t(key, el._i18nOrig[attr]);
                if (val == null) continue;
                if (target === null) {
                    if (el.textContent !== val) el.textContent = val;
                } else if (target === 'innerHTML') {
                    if (el.innerHTML !== val) el.innerHTML = val;
                } else {
                    if (el.getAttribute(target) !== val) el.setAttribute(target, val);
                }
            }
        }

        document.documentElement.lang = current;
        try {
            document.dispatchEvent(new CustomEvent('languagechange', { detail: { lang: current } }));
        } catch (e) {}
    }

    // Cambia de idioma, persiste y (re)aplica sin recargar.
    function setLang(lang) {
        if (!isSupported(lang)) lang = DEFAULT_LANG;
        var need = [];
        if (lang !== DEFAULT_LANG && !dict[lang]) need.push(lang);
        if (lang !== FALLBACK_LANG && !dict[FALLBACK_LANG]) need.push(FALLBACK_LANG);
        function finalize() {
            current = lang;
            try { localStorage.setItem(STORAGE_KEY, lang); } catch (e) {}
            apply();
        }
        if (need.length === 0) { finalize(); return; }
        Promise.all(need.map(loadLang)).then(finalize);
    }

    // Arranque inicial.
    function init() {
        current = detect();
        document.documentElement.lang = current;
        var need = [];
        if (current !== DEFAULT_LANG && !dict[current]) need.push(current);
        if (current !== FALLBACK_LANG && !dict[FALLBACK_LANG]) need.push(FALLBACK_LANG);
        if (need.length === 0) { apply(); return; }
        Promise.all(need.map(loadLang)).then(function () { apply(); });
    }

    window.i18n = {
        t: t,
        setLang: setLang,
        getLang: function () { return current; },
        supported: SUPPORTED.slice(),
        apply: apply,
        loadLang: loadLang
    };

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
