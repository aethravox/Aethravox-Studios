import { S } from './state.js';
import { openMessageMenu } from './Men__contextual_flotante.js';

    // ====== Bindear eventos a un mensaje (botón ⋯) ======
    export function bindMessageEvents(el) {
        const btn = el.querySelector('.chat-msg-menu-btn');
        if (!btn) return;
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            openMessageMenu(el, btn);
        });
    }

