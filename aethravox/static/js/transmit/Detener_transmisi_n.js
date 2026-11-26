import { S } from './state.js';
import { updateStatus } from './Status_pill.js';

    // ====== Detener transmisión ======
    export function stopSharing() {
        // Cerrar todos los PeerConnections
        S.peerConnections.forEach(pc => pc.close());
        S.peerConnections.clear();

        // Detener tracks locales
        if (S.localStream) {
            S.localStream.getTracks().forEach(t => t.stop());
            S.localStream = null;
        }

        // Avisar al server para cerrar la sala
        if (S.ws && S.ws.readyState === 1) {
            S.ws.send(JSON.stringify({ type: 'stop_room' }));
            setTimeout(() => { if (S.ws) S.ws.close(); }, 200);
            S.ws = null;
        }

        // Reset UI
        document.getElementById('sender-form-card').hidden = false;
        document.getElementById('sender-live-card').hidden = true;
        document.getElementById('sender-preview').hidden = true;
        document.getElementById('sender-preview').srcObject = null;
        document.getElementById('preview-empty').hidden = false;
        document.getElementById('receiver-count').textContent = '0';
        document.getElementById('receivers-list').innerHTML =
            '<p class="muted small">Cuando alguien se conecte, verás su nombre aquí.</p>';
        updateStatus('configurando', '#888');
        S.currentRoomName = null;
    }
    window.stopSharing = stopSharing;
    document.getElementById('stop-share-btn').addEventListener('click', stopSharing);

