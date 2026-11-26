import { S } from './state.js';
import { openMessageMenu } from './Men__contextual_flotante.js';

    // ====== Long-press en móvil para activar selección múltiple ======
    // En móvil NO hay botón ⋯ (los tres puntitos) — el menú de opciones aparece
    // al mantener presionado un mensaje. Esto también abre el menú contextual
    // (igual que en PC), y desde ahí se elige "Seleccionar".
    // En PC (mouse) el long-press no dispara nada: se sigue usando el botón ⋯.
    (function setupLongPress() {
        const msgs = document.getElementById('chat-messages');
        if (!msgs) return;
        let pressTimer = null;
        let pressedEl = null;
        let longPressed = false;
        const PRESS_MS = 450;  // tiempo para considerarse "mantener presionado"

        function clearPress() {
            if (pressTimer) { clearTimeout(pressTimer); pressTimer = null; }
            pressedEl = null;
        }
        // Solo Registrar en dispositivos táctiles (touchstart existe).
        // En desktop el click del botón ⋯ sigue siendo la vía principal.
        msgs.addEventListener('touchstart', (e) => {
            const el = e.target.closest('.chat-msg');
            if (!el || el.dataset.deleted === '1') return;
            pressedEl = el;
            longPressed = false;
            pressTimer = setTimeout(() => {
                longPressed = true;
                // Vibración si está disponible (feedback háptico en móvil).
                if (navigator.vibrate) navigator.vibrate(30);
                // Abrir el mismo menú contextual que el botón ⋯.
                // En móvil el botón ⋯ puede estar oculto o ser pequeño;
                // simulamos un "click" en él si existe, o abrimos el menú
                // directamente con un botón virtual.
                const btn = el.querySelector('.chat-msg-menu-btn');
                if (btn) {
                    openMessageMenu(el, btn);
                } else {
                    // Sin botón ⋯ (p.ej. modo edición): ignorar.
                    longPressed = false;
                }
            }, PRESS_MS);
        }, { passive: true });
        // Si el dedo se mueve mucho o se levanta antes, cancelar.
        msgs.addEventListener('touchmove', clearPress, { passive: true });
        msgs.addEventListener('touchend', (e) => {
            // Si fue long-press, evitar el click que vendría después
            // (no queremos que el click seleccione el mensaje recién tocado).
            if (longPressed) {
                e.preventDefault();
                longPressed = false;
            }
            clearPress();
        });
        msgs.addEventListener('touchcancel', clearPress, { passive: true });
    })();

