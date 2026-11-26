import { S } from './state.js';
import { hangupCall, startCall } from './Llamadas___videollamadas.js';
import { closeNewChatModal, switchTab } from './Modal__Nuevo_chat.js';
import { uploadImage } from './Subir_imagen.js';
import { sendMessage } from './Enviar_mensaje.js';

    // ====== Tono de llamada (WebAudio, sin assets externos) ======
    export function playRingTone() {
        try {
            if (!S.ringCtx) S.ringCtx = new (window.AudioContext || window.webkitAudioContext)();
            const beep = () => {
                if (!S.ringCtx) return;
                const osc = S.ringCtx.createOscillator();
                const gain = S.ringCtx.createGain();
                osc.frequency.value = 880;
                osc.type = 'sine';
                gain.gain.value = 0.0001;
                osc.connect(gain); gain.connect(S.ringCtx.destination);
                const t = S.ringCtx.currentTime;
                gain.gain.setValueAtTime(0.0001, t);
                gain.gain.exponentialRampToValueAtTime(0.18, t + 0.05);
                gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.5);
                osc.start(t); osc.stop(t + 0.55);
            };
            beep();
            S.ringInterval = setInterval(beep, 1500);
        } catch (e) { /* silencioso */ }
    }
    export function stopRingTone() {
        if (S.ringInterval) { clearInterval(S.ringInterval); S.ringInterval = null; }
    }

    // Botones del header: llamada de voz / video.
    document.getElementById('chat-call-voice').addEventListener('click', () => startCall('voice'));
    document.getElementById('chat-call-video').addEventListener('click', () => startCall('video'));

    // Botones de control dentro del overlay.
    document.getElementById('call-mute').addEventListener('click', () => {
        if (!S.callLocalStream) return;
        S.callMuted = !S.callMuted;
        S.callLocalStream.getAudioTracks().forEach(t => t.enabled = !S.callMuted);
        const btn = document.getElementById('call-mute');
        btn.textContent = S.callMuted ? '🔇' : '🎤';
        btn.classList.toggle('is-off', S.callMuted);
    });
    document.getElementById('call-cam').addEventListener('click', () => {
        if (!S.callLocalStream) return;
        S.callCamOff = !S.callCamOff;
        S.callLocalStream.getVideoTracks().forEach(t => t.enabled = !S.callCamOff);
        const btn = document.getElementById('call-cam');
        btn.textContent = S.callCamOff ? '🚫' : '📷';
        btn.classList.toggle('is-off', S.callCamOff);
    });
    document.getElementById('call-hang').addEventListener('click', () => hangupCall(null));

    // Si el usuario cierra la pestaña o se va, colgar la llamada activa.
    window.addEventListener('beforeunload', () => {
        if (S.callCurrentId && S.ws && S.ws.readyState === 1) {
            S.ws.send(JSON.stringify({
                type: 'call_end',
                call_id: S.callCurrentId,
                conversation_id: S.callPeerConvId,
            }));
        }
    });

    // Pegar imagen del portapapeles
    document.getElementById('chat-input').addEventListener('paste', async function(e) {
        const items = e.clipboardData?.items || [];
        for (const item of items) {
            if (item.type.startsWith('image/')) {
                const file = item.getAsFile();
                if (file) {
                    e.preventDefault();
                    await uploadImage(file);
                    return;
                }
            }
        }
    });
    document.getElementById('new-chat-modal').addEventListener('click', (e) => {
        if (e.target.id === 'new-chat-modal') closeNewChatModal();
    });
    document.querySelectorAll('.chat-new-tab').forEach(tab => {
        tab.addEventListener('click', () => switchTab(tab.dataset.tab));
    });

    // Input: Enter para enviar, Shift+Enter para salto, auto-resize
    S.input.addEventListener('keydown', e => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            sendMessage();
        }
    });
    // FIX Android: al recibir foco el textarea, hacemos scroll solo del
    // contenedor de mensajes hacia abajo. NO usamos scrollIntoView del input
    // porque eso empujaría el chat-header (con el nombre) fuera de la vista.
    // El 100dvh del CSS ya se encarga de que la barra quede visible.
    S.input.addEventListener('focus', () => {
        requestAnimationFrame(() => {
            const msgs = document.getElementById('chat-messages');
            if (msgs) msgs.scrollTop = msgs.scrollHeight;
        });
    });
    S.input.addEventListener('input', () => {
        S.input.style.height = 'auto';
        S.input.style.height = Math.min(S.input.scrollHeight, 200) + 'px';
        // Send typing indicator
        if (S.currentConv && S.ws && S.ws.readyState === 1 && !S.isTyping) {
            S.isTyping = true;
            S.ws.send(JSON.stringify({ type: 'typing', conversation_id: S.currentConv }));
            clearTimeout(S.typingTimer);
            S.typingTimer = setTimeout(() => { S.isTyping = false; }, 2000);
        } else if (S.isTyping) {
            clearTimeout(S.typingTimer);
            S.typingTimer = setTimeout(() => { S.isTyping = false; }, 2000);
        }
    });
    // FIX Android: cuando cambia el tamaño del viewport (teclado que aparece/desaparece),
    // recolocar el scroll de mensajes abajo para que no se vea "cortado".
    if (window.visualViewport) {
        window.visualViewport.addEventListener('resize', () => {
            const msgs = document.getElementById('chat-messages');
            if (msgs && document.activeElement === S.input) {
                msgs.scrollTop = msgs.scrollHeight;
            }
        });
    }

    // Cerrar modal con Esc
    document.addEventListener('keydown', e => {
        if (e.key === 'Escape') {
            const modal = document.getElementById('new-chat-modal');
            if (!modal.hidden) closeNewChatModal();
        }
    });

