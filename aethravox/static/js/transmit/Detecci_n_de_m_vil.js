import { S } from './state.js';
import { syncShareButton } from './Bot_n_din_mico.js';

    // ====== Detección de móvil: si getDisplayMedia no está disponible,
    // ocultamos la opción "Pantalla" y dejamos "Cámara" como única opción. ======
    (function autoDetectSource() {
        const sourceSel = document.getElementById('source-select');
        const hint = document.getElementById('source-hint');
        const screenOpt = sourceSel.querySelector('option[value="screen"]');
        const canScreen = !!(navigator.mediaDevices && navigator.mediaDevices.getDisplayMedia);
        if (!canScreen && screenOpt) {
            screenOpt.disabled = true;
            screenOpt.textContent = '🖥️ Pantalla (no soportado en este navegador)';
            sourceSel.value = 'camera';
            if (hint) {
                hint.innerHTML = '<strong>Este navegador no permite compartir pantalla.</strong> ' +
                    'Transmite usando la cámara del dispositivo (móvil, tablet o webcam). ' +
                    'Para compartir pantalla usa un PC de escritorio.';
            }
        }
        // Sincronizar el botón con el valor inicial (puede haber cambiado arriba).
        syncShareButton();
    })();

