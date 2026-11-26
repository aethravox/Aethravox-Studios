import { S } from './state.js';
import { formatDuration } from './Helpers_de_formato.js';
import { showToast } from './Utilidades.js';

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

    // ====== Subir archivo genérico al chat (cualquier MIME) ======
    export async function uploadFile(file, kind) {
        if (!S.currentConv) { showToast('Selecciona una conversación primero', 'warning'); return; }
        const MAX = 60 * 1024 * 1024;
        if (file.size > MAX) {
            showToast('Archivo demasiado grande (máximo 60MB)', 'error');
            return;
        }
        const formData = new FormData();
        formData.append('file', file);
        formData.append('kind', kind || 'file');
        formData.append('name', file.name || 'archivo');
        try {
            const resp = await fetch('/api/chat/' + S.currentConv + '/upload_file', {
                method: 'POST',
                body: formData,
            });
            if (!resp.ok) {
                const err = await resp.json().catch(() => ({}));
                showToast(err.error || 'Error al subir archivo', 'error');
            }
            // El mensaje llega por WS broadcast.
        } catch (e) {
            showToast('Error de red al subir archivo', 'error');
        }
    }
    window.uploadFile = uploadFile;

    // ====== Nota de voz con MediaRecorder ======
    // Pulsar 🎙️ inicia grabación; pulsar ⏹ detiene y envía. ✕ cancela.
    // En móvil usa audio/webm si está disponible, si no audio/mp4. Si el
    // navegador no soporta MediaRecorder avisamos y no hacemos nada.

    export async function startVoiceRecording() {
        if (!S.currentConv) { showToast('Selecciona una conversación primero', 'warning'); return; }
        if (S.voiceRecorder) return;
        if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
            showToast('Tu navegador no permite grabar audio', 'warning');
            return;
        }
        if (!window.MediaRecorder) {
            showToast('Tu navegador no soporta notas de voz', 'warning');
            return;
        }
        try {
            S.voiceStream = await navigator.mediaDevices.getUserMedia({ audio: true });
        } catch (err) {
            console.error('getUserMedia audio:', err);
            showToast('No se pudo acceder al micrófono. Revisa los permisos.', 'warning');
            return;
        }
        // Elegir mimeType soportado. Orden preferido para máxima compatibilidad
        // con el <audio controls> del receptor (Chrome/Safari/Firefox).
        const candidates = [
            'audio/webm;codecs=opus',
            'audio/webm',
            'audio/ogg;codecs=opus',
            'audio/mp4',
            'audio/mpeg',
        ];
        let pickedMime = '';
        for (const c of candidates) {
            if (MediaRecorder.isTypeSupported && MediaRecorder.isTypeSupported(c)) {
                pickedMime = c; break;
            }
        }
        try {
            S.voiceRecorder = new MediaRecorder(S.voiceStream, pickedMime ? { mimeType: pickedMime } : undefined);
        } catch (e) {
            console.error('MediaRecorder ctor:', e);
            showToast('No se pudo iniciar la grabación', 'error');
            stopVoiceStream();
            return;
        }
        S.voiceChunks = [];
        S.voiceRecorder.ondataavailable = (e) => {
            if (e.data && e.data.size > 0) S.voiceChunks.push(e.data);
        };
        S.voiceRecorder.onstop = async () => {
            const blob = new Blob(S.voiceChunks, { type: S.voiceRecorder.mimeType || 'audio/webm' });
            const durationSec = Math.max(1, Math.round((Date.now() - S.voiceStartTime) / 1000));
            stopVoiceStream();
            S.voiceRecorder = null;
            S.voiceChunks = [];
            stopVoiceTimer();
            // Si el blob es muy pequeño o vacío, lo descartamos.
            if (!blob || blob.size < 500) { showToast('Grabación demasiado corta', 'warning'); return; }
            const ext = mimeToExtForVoice(blob.type);
            const file = new File([blob], 'nota-de-voz.' + ext, { type: blob.type });
            // Subir con kind=voice y duration.
            await uploadVoice(file, durationSec);
        };
        S.voiceRecorder.start();
        S.voiceStartTime = Date.now();
        startVoiceTimer();
        document.getElementById('chat-voice-recording').hidden = false;
        document.getElementById('chat-voice-btn').classList.add('is-recording');
    }
    export function stopVoiceStream() {
        if (S.voiceStream) {
            S.voiceStream.getTracks().forEach(t => t.stop());
            S.voiceStream = null;
        }
    }
    export function startVoiceTimer() {
        const el = document.getElementById('chat-voice-rec-time');
        if (!el) return;
        S.voiceTimerInterval = setInterval(() => {
            const secs = Math.floor((Date.now() - S.voiceStartTime) / 1000);
            el.textContent = formatDuration(secs);
        }, 250);
    }
    export function stopVoiceTimer() {
        if (S.voiceTimerInterval) { clearInterval(S.voiceTimerInterval); S.voiceTimerInterval = null; }
        document.getElementById('chat-voice-recording').hidden = true;
        document.getElementById('chat-voice-btn').classList.remove('is-recording');
    }
    export function stopVoiceRecording() {
        if (S.voiceRecorder && S.voiceRecorder.state !== 'inactive') {
            S.voiceRecorder.stop();
        }
    }
    export function cancelVoiceRecording() {
        if (S.voiceRecorder && S.voiceRecorder.state !== 'inactive') {
            // Marcar para que onstop no suba nada: vaciamos los chunks.
            S.voiceChunks = [];
            S.voiceRecorder.onstop = () => {
                stopVoiceStream();
                S.voiceRecorder = null;
                stopVoiceTimer();
            };
            S.voiceRecorder.stop();
        } else {
            stopVoiceTimer();
        }
    }
    export function mimeToExtForVoice(mime) {
        if (mime.includes('webm')) return 'weba';
        if (mime.includes('ogg')) return 'oga';
        if (mime.includes('mp4') || mime.includes('m4a')) return 'm4a';
        if (mime.includes('mpeg')) return 'mp3';
        return 'weba';
    }
    export async function uploadVoice(file, durationSec) {
        if (!S.currentConv) return;
        const formData = new FormData();
        formData.append('file', file);
        formData.append('kind', 'voice');
        formData.append('duration', String(durationSec));
        formData.append('name', file.name);
        try {
            const resp = await fetch('/api/chat/' + S.currentConv + '/upload_file', {
                method: 'POST',
                body: formData,
            });
            if (!resp.ok) {
                const err = await resp.json().catch(() => ({}));
                showToast(err.error || 'Error al subir nota de voz', 'error');
            }
        } catch (e) {
            showToast('Error de red al subir nota de voz', 'error');
        }
    }
    document.getElementById('chat-voice-btn').addEventListener('click', () => {
        if (S.voiceRecorder) {
            // Si ya está grabando, clic = detener y enviar.
            stopVoiceRecording();
        } else {
            startVoiceRecording();
        }
    });
    document.getElementById('chat-voice-stop').addEventListener('click', stopVoiceRecording);
    document.getElementById('chat-voice-cancel').addEventListener('click', cancelVoiceRecording);

