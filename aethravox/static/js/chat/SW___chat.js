import { S } from './state.js';
import { selectConv } from './Seleccionar_conversaci_n.js';

    // ====== SW → chat: al clickar una notificación push/in-page, el SW nos
    // avía para abrir la conversación correspondiente (notificationclick). ======
    if ('serviceWorker' in navigator) {
        navigator.serviceWorker.addEventListener('message', function(event) {
            var data = event.data || {};
            if (data.type === 'notification-clicked' && data.conversation_id) {
                // Darle un tick a que la lista de convs esté cargada.
                selectConv(data.conversation_id);
            }
        });
    }

