import { S } from './state.js';
import { stopSharing } from './Detener_transmisi_n.js';
import { handleWs } from './Recibir_WS.js';
import { showToast } from './Utilidades.js';

    // ====== Conectar WS ======
    export function connectWS() {
        const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
        S.ws = new WebSocket(proto + '//' + location.host + '/transmitir/ws');
        S.ws.onmessage = (e) => {
            try { handleWs(JSON.parse(e.data)); }
            catch (err) { console.error('WS parse:', err); }
        };
        S.ws.onclose = () => {
            console.log('WS cerrado');
            // No reconectar automáticamente si fue intencional.
            // Solo cambiar UI si estábamos transmitiendo.
            if (S.localStream) {
                showToast('Conexión perdida. Deteniendo…', 'warning');
                stopSharing();
            }
        };
        S.ws.onerror = (e) => console.error('WS error:', e);
    }

