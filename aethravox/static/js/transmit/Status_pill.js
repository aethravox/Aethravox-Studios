import { S } from './state.js';

    // ====== Status pill ======
    export function updateStatus(text, color) {
        const pill = document.getElementById('sender-status');
        const dot = pill.querySelector('.transmit-status-dot');
        const t = pill.querySelector('.transmit-status-text');
        if (dot) dot.style.background = color;
        if (t) t.textContent = text;
    }

    // ====== Salir limpio al cerrar pestaña ======
    window.addEventListener('beforeunload', () => {
        if (localStream) {
            // Enviar stop_room antes de cerrar
            if (ws && ws.readyState === 1) {
                ws.send(JSON.stringify({ type: 'stop_room' }));
            }
            localStream.getTracks().forEach(t => t.stop());
        }
    });
