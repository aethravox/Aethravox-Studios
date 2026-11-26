import { S } from './state.js';

    // ====== Botón dinámico: cambia el texto según la fuente elegida ======
    // "Compartir pantalla" → 📺  |  "Compartir cámara" → 📷
    export function syncShareButton() {
        const sel = document.getElementById('source-select');
        const btn = document.getElementById('start-share-btn');
        if (!sel || !btn) return;
        if (sel.value === 'camera') {
            btn.textContent = '📷 Compartir cámara';
        } else {
            btn.textContent = '📺 Compartir pantalla';
        }
    }
    document.getElementById('source-select').addEventListener('change', syncShareButton);

