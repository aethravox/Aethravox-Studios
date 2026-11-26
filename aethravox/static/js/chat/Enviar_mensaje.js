import { S } from './state.js';
import { updateActionSlot } from './Nota_de_voz.js';

    // ====== Enviar mensaje ======
    export function sendMessage() {
        const input = document.getElementById('chat-input');
        const body = S.input.value.trim();
        if (!body || !S.currentConv || !S.ws || S.ws.readyState !== 1) return;
        S.ws.send(JSON.stringify({ type: 'message', conversation_id: S.currentConv, body: body }));
        // Stop typing
        if (S.isTyping) {
            S.isTyping = false;
            clearTimeout(S.typingTimer);
        }
        S.input.value = '';
        S.input.style.height = 'auto';
        // Tras enviar, el textbox vuelve a estar vacío: mostrar 🎙️ (voz), ocultar ➤.
        updateActionSlot();
    }
    window.sendMessage = sendMessage;

