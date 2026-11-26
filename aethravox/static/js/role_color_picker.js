/* Aethravox Studios — Color picker reutilizable.
 *
 * Inicializa TODOS los .color-picker de la página. Cada picker tiene:
 *   - un <input type="hidden" name="color"> donde deja el composite final
 *     "<bg>|<text>|<opacity>" (bg = "#hex" sólido o "#hex1~#hex2" degradado).
 *   - una vista previa (.cp-prev-badge) que refleja el color en vivo.
 *   - controles: tipo de fondo (sólido/degradado), color principal, color
 *     secundario (solo degradado), texto del badge (auto o manual), opacidad.
 *
 * Lo usan /admin/roles (crear + editar) y /admin/títulos (crear + editar).
 * El backend lee el composite con role_color::parse_role_color y genera el
 * CSS del badge en /role-colors.css o /title-colors.css.
 */
(function () {
    'use strict';

    var SWATCHES = [
        '#e4b022', '#f59e0b', '#ef4444', '#ec4899', '#8b5cf6',
        '#3b82f6', '#06b6d4', '#10b981', '#22c55e', '#84cc16',
        '#ffffff', '#9ca3af', '#374151', '#111827'
    ];

    function hexToRgb(hex) {
        var m = /^#?([0-9a-f]{6})$/i.exec((hex || '').trim());
        if (!m) return null;
        var n = parseInt(m[1], 16);
        return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
    }

    function relLum(rgb) {
        if (!rgb) return 0;
        var a = [rgb.r, rgb.g, rgb.b].map(function (v) {
            v /= 255;
            return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
        });
        return 0.2126 * a[0] + 0.7152 * a[1] + 0.0722 * a[2];
    }

    function autoText(bgHex) {
        var rgb = hexToRgb(bgHex);
        if (!rgb) return '#ffffff';
        return relLum(rgb) > 0.45 ? '#111827' : '#ffffff';
    }

    function buildSwatches(container) {
        SWATCHES.forEach(function (c) {
            var b = document.createElement('button');
            b.type = 'button';
            b.className = 'cp-swatch';
            b.style.background = c;
            b.title = c;
            b.dataset.color = c;
            container.appendChild(b);
        });
    }

    function initPicker(root) {
        // Evita doble inicialización de listeners/swatches (DOMContentLoaded +
        // reinit al abrir modal). Si ya estaba inicializado, solo releemos el
        // hidden (el modal de "Editar" lo rellena justo antes de abrirse).
        if (root.dataset.cpInit === '1') {
            var reload = root.__cpLoad;
            if (typeof reload === 'function') reload();
            return;
        }
        root.dataset.cpInit = '1';
        var hidden = root.querySelector('input[type="hidden"][name="color"]');
        var preview = root.querySelector('.cp-prev-badge');
        var bgType = root.querySelector('.cp-bg-type');
        var c2Row = root.querySelector('.cp-c2');
        var opacity = root.querySelector('.cp-opacity');
        var opacityOut = root.querySelector('.cp-opacity-out');
        var textAuto = root.querySelector('.cp-text-auto');

        var slots = {};
        ['c1', 'c2', 'text'].forEach(function (slot) {
            slots[slot] = {
                hex: root.querySelector('.cp-hex[data-slot="' + slot + '"]'),
                native: root.querySelector('.cp-native[data-slot="' + slot + '"]'),
                swatches: root.querySelector('.cp-swatches[data-slot="' + slot + '"]')
            };
        });

        // Semilla inicial desde el hidden (p.ej. al editar un rol/título existente).
        // Formato composite: "<bg>|<text>|<opacity>" donde bg = "#hex" o "#hex1~#hex2".
        function loadFromHidden() {
            var raw = (hidden.value || '').trim();
            if (!raw) return;
            var parts = raw.split('|');
            var bg = parts[0] || '';
            var txt = parts[1] || '';
            var op = parseInt(parts[2], 10);
            if (bg.indexOf('~') !== -1) {
                var pair = bg.split('~');
                setHex('c1', pair[0]);
                setHex('c2', pair[1] || pair[0]);
                if (bgType) bgType.value = 'gradient';
                if (c2Row) c2Row.hidden = false;
            } else {
                setHex('c1', bg);
                if (bgType) bgType.value = 'solid';
                if (c2Row) c2Row.hidden = true;
            }
            if (txt) {
                setHex('text', txt);
                if (textAuto) textAuto.checked = false;
            }
            if (!isNaN(op)) {
                if (opacity) opacity.value = op;
                if (opacityOut) opacityOut.textContent = op;
            }
        }

        function setHex(slot, hex) {
            var s = slots[slot];
            if (!s) return;
            hex = (hex || '').trim();
            if (s.hex) s.hex.value = hex;
            if (s.native) s.native.value = hex;
        }

        function getHex(slot) {
            var s = slots[slot];
            return s && s.hex ? (s.hex.value || '').trim() : '';
        }

        function sync() {
            var c1 = getHex('c1');
            var c2 = getHex('c2');
            var isGrad = bgType && bgType.value === 'gradient';
            var bg;
            if (!c1) {
                bg = '';
            } else if (isGrad && c2) {
                bg = c1 + '~' + c2;
            } else {
                bg = c1;
            }
            var txt;
            if (textAuto && textAuto.checked) {
                txt = autoText(c1);
                setHex('text', txt);
                if (slots.text.hex) slots.text.hex.disabled = true;
                if (slots.text.native) slots.text.native.disabled = true;
            } else {
                txt = getHex('text');
                if (slots.text.hex) slots.text.hex.disabled = false;
                if (slots.text.native) slots.text.native.disabled = false;
            }
            var op = opacity ? parseInt(opacity.value, 10) : 100;
            if (isNaN(op)) op = 100;
            if (opacityOut) opacityOut.textContent = op;

            // Composite final en el hidden.
            hidden.value = bg ? (bg + '|' + txt + '|' + op) : '';

            // Vista previa en vivo.
            if (preview && bg) {
                var alpha = op / 100;
                var rgba1 = hexToRgba(c1, alpha);
                if (isGrad && c2) {
                    var rgba2 = hexToRgba(c2, alpha);
                    preview.style.background = 'linear-gradient(90deg,' + rgba1 + ',' + rgba2 + ')';
                } else {
                    preview.style.background = rgba1;
                }
                preview.style.color = txt || autoText(c1);
                preview.style.opacity = '1';
            } else if (preview) {
                preview.style.background = '';
                preview.style.color = '';
            }
        }

        function hexToRgba(hex, alpha) {
            var rgb = hexToRgb(hex);
            if (!rgb) return hex;
            return 'rgba(' + rgb.r + ',' + rgb.g + ',' + rgb.b + ',' + alpha + ')';
        }

        // Build swatches una vez.
        ['c1', 'c2', 'text'].forEach(function (slot) {
            if (slots[slot].swatches) buildSwatches(slots[slot].swatches);
        });

        // Eventos.
        ['c1', 'c2', 'text'].forEach(function (slot) {
            var s = slots[slot];
            if (!s) return;
            if (s.hex) s.hex.addEventListener('input', sync);
            if (s.native) {
                s.native.addEventListener('input', function () {
                    setHex(slot, s.native.value);
                    sync();
                });
            }
            if (s.swatches) {
                s.swatches.addEventListener('click', function (e) {
                    var t = e.target.closest('.cp-swatch');
                    if (!t) return;
                    setHex(slot, t.dataset.color);
                    sync();
                });
            }
        });
        if (bgType) bgType.addEventListener('change', function () {
            if (c2Row) c2Row.hidden = bgType.value !== 'gradient';
            sync();
        });
        if (opacity) opacity.addEventListener('input', sync);
        if (textAuto) textAuto.addEventListener('change', sync);

        // Semilla inicial y primer sync.
        loadFromHidden();
        sync();

        // Expone la recarga para que los modales de /admin/títulos y
        // /admin/roles puedan releer el hidden al re-abrir (caso "Editar"):
        // el modal setea hidden.value y luego llama a window.__reinitColorPicker().
        root.__cpLoad = function () {
            // Reset previo por si el hidden quedó vacío al crear uno nuevo.
            ['c1', 'c2', 'text'].forEach(function (slot) {
                var s = slots[slot];
                if (s && s.hex) s.hex.value = '';
                if (s && s.native) s.native.value = '#000000';
            });
            if (textAuto) textAuto.checked = true;
            if (opacity) { opacity.value = 100; }
            if (opacityOut) opacityOut.textContent = '100';
            if (bgType) bgType.value = 'solid';
            if (c2Row) c2Row.hidden = true;
            loadFromHidden();
            sync();
        };
    }

    function initAll() {
        document.querySelectorAll('.color-picker').forEach(initPicker);
    }

    // Re-inicializa los pickers. Lo usan los modales de /admin/títulos y
    // /admin/roles cuando se abren y rellenan el hidden `color` DESPUÉS del
    // DOMContentLoaded (caso "Editar"). Sin esto, el picker mostraría los
    // valores por defecto en vez del color actual del título/rol.
    window.__reinitColorPicker = function () {
        document.querySelectorAll('.color-picker').forEach(initPicker);
    };

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initAll);
    } else {
        initAll();
    }
})();
