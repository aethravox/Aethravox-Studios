import { S } from './state.js';
import { showCallButtons } from './Llamadas___videollamadas.js';

    // ====== Botón "volver" en móvil ======
    document.getElementById('chat-back-mobile').addEventListener('click', () => {
        document.getElementById('chat-app').classList.remove('chat-open');
        S.currentConv = null;  // ← resetear: si llega un mensaje, no se marca como leído
        // Ocultar botón de selección múltiple (no hay conv abierta).
        const selectToggle = document.getElementById('chat-select-toggle');
        if (selectToggle) selectToggle.hidden = true;
        // Ocultar botones de llamada (no hay conv abierta).
        showCallButtons(false);
        if (window.exitSelectMode) window.exitSelectMode();
    });

