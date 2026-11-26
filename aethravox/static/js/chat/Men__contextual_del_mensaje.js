import { S } from './state.js';
import { escapeHtml } from './Utilidades.js';
import { bindSelectClicks } from './Modo_selecci_n_m_ltiple.js';
import { formatBody } from './Body_del_mensaje.js';
import { bindMessageEvents } from './Bindear_eventos_a_un_mensaje.js';
import { checkMarks } from './Estado_del_mensaje.js';

    // ====== Menú contextual del mensaje (tres puntitos) ======
    // Devuelve el HTML del botón ⋯ solo si el mensaje no está borrado.
    // El evento click se bindea en bindMessageEvents.
    export function messageMenuButton(m, isSelf) {
        if (m.deleted) return '';
        return `<button class="chat-msg-menu-btn" title="Más opciones">⋯</button>`;
    }

    export function renderMessages(msgs) {
        const c = document.getElementById('chat-messages');
        if (!msgs || msgs.length === 0) {
            c.innerHTML = '<div class="chat-empty"><p class="muted">No hay mensajes todavía. ¡Escribe el primero!</p></div>';
            return;
        }
        c.innerHTML = msgs.map(m => buildMessageHtml(m)).join('');
        // Bindear eventos (click en ⋯, etc.) a cada mensaje
        c.querySelectorAll('.chat-msg').forEach(el => bindMessageEvents(el));
        c.scrollTop = c.scrollHeight;
        // Si estábamos en modo selección, re-aplicar la clase al contenedor
        // (innerHTML la pierde) y re-bindear clicks de selección.
        if (S.selectMode) {
            c.classList.add('chat-select-mode');
            bindSelectClicks();
            // Re-marcar los que estaban seleccionados.
            S.selectedIds.forEach(id => {
                const el = c.querySelector('.chat-msg[data-id="' + id + '"]');
                if (el) el.classList.add('is-selected');
            });
        }
    }

    // Construye el HTML de un mensaje. Usado por renderMessages() y por el
    // append de un nuevo mensaje entrante via WS.
    export function buildMessageHtml(m) {
        const isSelf = !!m.is_self;
        const cls = isSelf ? 'sent' : 'received';
        const senderName = (!isSelf && m.sender_name)
            ? `<div class="chat-msg-sender">${escapeHtml(m.sender_name)}</div>`
            : '';
        const checks = checkMarks(m.status || 'sent', isSelf);
        const menuBtn = messageMenuButton(m, isSelf);
        const bodyHtml = formatBody(m.body || '', m.deleted, m.edited, isSelf);
        const timeStr = m.edited_at && !m.deleted
            ? `${escapeHtml(m.edited_at)} ${checks}`
            : `${escapeHtml(m.time || m.edited_at || '')} ${checks}`;
        return `<div class="chat-msg ${cls}" data-id="${m.id}" data-self="${isSelf ? '1' : '0'}" data-sender="${m.sender_id || 0}" data-deleted="${m.deleted ? '1' : '0'}" data-created-iso="${escapeHtml(m.created_iso || '')}">
            <div class="chat-msg-content">
                ${senderName}
                <div class="chat-msg-body">${bodyHtml}</div>
                <div class="chat-msg-time">${timeStr}</div>
            </div>
            ${menuBtn}
        </div>`;
    }

