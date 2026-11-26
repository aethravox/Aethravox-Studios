import { S } from './state.js';
import { handleWs } from './Recibir_mensajes_WS.js';
import { loadConversations } from './Cargar_conversaciones.js';

    // ====== WebSocket ======
    export function connectWS() {
        const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
        S.ws = new WebSocket(proto + '//' + location.host + '/chat/ws');
        S.ws.onmessage = async (e) => {
            try { await handleWs(JSON.parse(e.data)); }
            catch (err) { console.error('WS parse error:', err); }
        };
        S.ws.onclose = () => {
            console.log('WS cerrado, reintentando en 3s…');
            setTimeout(connectWS, 3000);
        };
        S.ws.onopen = async () => {
            console.log('WS conectado');
            await loadConversations();
        };
        S.ws.onerror = (e) => console.error('WS error:', e);
    }

