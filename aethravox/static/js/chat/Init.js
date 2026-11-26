import { S } from './state.js';
import { connectWS } from './WebSocket.js';
import { loadConversations } from './Cargar_conversaciones.js';

    // ====== Init ======
    connectWS();
    // Al entrar a /chat mostramos SIEMPRE la lista de conversaciones
    // (recientes), sin reabrir automáticamente el último chat. Antes esto leía
    // localStorage['aethravox_last_conv'] y lo abría, lo que hacía que el
    // usuario nunca viera "recientes" al recargar — y cada intento de arreglarlo
    // rompía otra cosa. Ahora no guardamos nada: la selección es efímera.
    // Cargar conversaciones inmediatamente (no esperar a WS).
    loadConversations().then(() => {
        if (S.conversations.length === 0) {
            // Retry si la primera carga vino vacía (WS quizás aún no conectaba).
            setTimeout(() => { loadConversations(); }, 1000);
        }
    });
