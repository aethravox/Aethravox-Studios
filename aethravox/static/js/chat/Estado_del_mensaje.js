import { S } from './state.js';

    // ====== Estado del mensaje (estilo texto, no símbolos) ======
    export function checkMarks(status, isSelf) {
        if (!isSelf) return '';
        if (status === 'read') return '<span class="check-text check-read">leído</span>';
        if (status === 'delivered') return '<span class="check-text check-delivered">entregado</span>';
        return '<span class="check-text check-sent">enviado</span>';
    }

