import { S } from './state.js';

    // ====== Alternar botón Enviar / Nota de voz según el contenido del textbox ======
    // Vacío → muestra 🎙️ (nota de voz). Con texto → muestra ➤ (enviar).
    // Ambos ocupan el mismo slot derecho de la barra de input.
    export function updateActionSlot() {
        const input = document.getElementById('chat-input');
        const sendBtn = document.getElementById('chat-send');
        const voiceBtn = document.getElementById('chat-voice-btn');
        if (!S.input || !sendBtn || !voiceBtn) return;
        const hasText = S.input.value.trim().length > 0;
        sendBtn.hidden = !hasText;
        voiceBtn.hidden = hasText;
        // Si estamos grabando y el usuario empieza a escribir, NO cortamos la
        // grabación: el usuario puede seguir grabando y luego decidir. Solo
        // ocultamos el botón de voz visualmente (la pill de grabación sigue
        // visible encima del input).
    }
    document.getElementById('chat-input').addEventListener('input', updateActionSlot);
    // Estado inicial: vacío → voz visible, enviar oculto.
    updateActionSlot();
    window.updateActionSlot = updateActionSlot;

