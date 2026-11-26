import { S } from './state.js';
import { escapeHtml, showToast } from './Utilidades.js';
import { handleCallAccepted, handleCallEnd, handleCallIce, handleCallIncoming, handleCallRejected, handleCallRinging, handleCallSdp, handleCallUnavailable } from './Llamadas___videollamadas.js';
import { selectConv } from './Seleccionar_conversaci_n.js';
import { formatBody } from './Body_del_mensaje.js';
import { buildMessageHtml } from './Men__contextual_del_mensaje.js';
import { loadConversations } from './Cargar_conversaciones.js';
import { bindMessageEvents } from './Bindear_eventos_a_un_mensaje.js';
import { checkMarks } from './Estado_del_mensaje.js';

    // ====== Recibir mensajes WS ======
    export async function handleWs(d) {
        if (d.type === 'message') {
            if (d.conversation_id === S.currentConv) {
                const c = document.getElementById('chat-messages');
                // Usar buildMessageHtml para que el mensaje tenga el botón ⋯
                // y todos los data attributes.
                const tmp = document.createElement('div');
                tmp.innerHTML = buildMessageHtml(d);
                const el = tmp.firstElementChild;
                if (el) {
                    bindMessageEvents(el);
                    c.appendChild(el);
                    c.scrollTop = c.scrollHeight;
                }
                // Si NO soy el sender → marcar como leído ANTES de recargar la lista
                if (!d.is_self) {
                    await fetch('/api/chat/read/' + S.currentConv, { method: 'POST' });
                }
            }
            // Notificación de escritorio si el mensaje NO es mío Y:
            //   - viene en una conversación que NO es la abierta ahora, O
            //   - la pestaña no está enfocada (aunque sea la conv abierta).
            // Así el usuario se entera aunque esté en otra pestaña o app.
            if (!d.is_self && window.showDesktopNotification) {
                const inCurrentConv = d.conversation_id === S.currentConv;
                const tabFocused = document.hasFocus();
                if (!inCurrentConv || !tabFocused) {
                    // Previsualizar el body: los mensajes especiales ([IMAGE]/[VOICE]/[FILE])
                    // se muestran como texto legible en vez del código interno.
                    const rawBody = String(d.body || '');
                    let bodyPreview;
                    if (rawBody.startsWith('[IMAGE]')) bodyPreview = '📷 Imagen';
                    else if (rawBody.startsWith('[VOICE]')) bodyPreview = '🎙️ Nota de voz';
                    else if (rawBody.startsWith('[FILE]')) bodyPreview = '📎 Archivo';
                    else bodyPreview = rawBody;
                    // Truncar body largo para que no ocupe toda la notificación.
                    if (bodyPreview.length > 120) bodyPreview = bodyPreview.slice(0, 117) + '…';
                    const senderName = d.sender_name || 'Mensaje nuevo';
                    // Si el mensaje NO es de la conv abierta, forzamos la notificación
                    // aunque la pestaña esté enfocada (el usuario no está viendo esa conv).
                    window.showDesktopNotification(senderName, bodyPreview, {
                        tag: 'msg-' + d.conversation_id,
                        always: !inCurrentConv,
                        url: '/chat',
                        conversation_id: d.conversation_id,
                        type: 'message',
                        onclick: function() {
                            // Fallback para notificaciones page-owned (sin SW):
                            // el SW gestiona el click vía notificationclick + postMessage.
                            if (typeof selectConv === 'function') selectConv(d.conversation_id);
                        },
                    });
                }
            }
            // Siempre refrescar la lista de conversaciones (recientes) para que
            // el último mensaje se vea sin recargar la página.
            await loadConversations();
        } else if (d.type === 'message_edited') {
            // Un mensaje fue editado. Si estamos viendo esa conversación, actualizar
            // el body + mostrar badge "editado".
            if (d.conversation_id === S.currentConv) {
                const el = document.querySelector('.chat-msg[data-id="' + d.id + '"]');
                if (el) {
                    const bodyEl = el.querySelector('.chat-msg-body');
                    const timeEl = el.querySelector('.chat-msg-time');
                    if (bodyEl && d.body !== undefined) {
                        // Re-formatear body (puede ser texto o imagen)
                        bodyEl.innerHTML = formatBody(d.body, false, true, el.dataset.self === '1');
                    }
                    if (timeEl && d.edited_at) {
                        const isSelf = el.dataset.self === '1';
                        const checks = checkMarks('read', isSelf);
                        // Mantener checks actuales; si el mensaje era del usuario,
                        // respetar el estado. Para simplicidad mostramos edited_at + checks.
                        const existingChecks = timeEl.querySelector('.check-text');
                        const checksHtml = existingChecks ? existingChecks.outerHTML : checks;
                        timeEl.innerHTML = escapeHtml(d.edited_at) + ' ' + checksHtml;
                    }
                }
            }
            await loadConversations();
        } else if (d.type === 'message_deleted') {
            // Un mensaje fue borrado. Actualizar DOM.
            if (d.conversation_id === S.currentConv) {
                const el = document.querySelector('.chat-msg[data-id="' + d.id + '"]');
                if (el) {
                    el.dataset.deleted = '1';
                    const bodyEl = el.querySelector('.chat-msg-body');
                    if (bodyEl) bodyEl.innerHTML = formatBody('', true, false, el.dataset.self === '1');
                    // Quitar el botón ⋯ porque ya no hay acciones disponibles.
                    const btn = el.querySelector('.chat-msg-menu-btn');
                    if (btn) btn.remove();
                    const timeEl = el.querySelector('.chat-msg-time');
                    if (timeEl) {
                        const checks = timeEl.querySelector('.check-text');
                        timeEl.innerHTML = '<span class="muted">eliminado</span>' + (checks ? ' ' + checks.outerHTML : '');
                    }
                }
            }
            await loadConversations();
        } else if (d.type === 'typing') {
            if (d.conversation_id === S.currentConv) {
                const indicator = document.getElementById('typing-indicator');
                if (indicator) {
                    indicator.textContent = 'escribiendo...';
                    clearTimeout(S.typingTimeout);
                    S.typingTimeout = setTimeout(() => {
                        if (indicator) indicator.textContent = '';
                    }, 3000);
                }
            }
        } else if (d.type === 'status_update') {
            // Alguien leyó los mensajes → cambiar "entregado"/"enviado" a "leído" en tiempo real
            if (d.conversation_id === S.currentConv && d.status === 'read') {
                const c = document.getElementById('chat-messages');
                c.querySelectorAll('.chat-msg.sent .check-delivered').forEach(el => {
                    el.classList.remove('check-delivered');
                    el.classList.add('check-read');
                    el.textContent = 'leído';
                });
                c.querySelectorAll('.chat-msg.sent .check-sent').forEach(el => {
                    el.classList.remove('check-sent');
                    el.classList.add('check-read');
                    el.textContent = 'leído';
                });
            }
        } else if (d.type === 'error') {
            // Mensaje de error del backend (ej: sin permiso para enviar)
            if (d.message) showToast(d.message, 'warning');
        } else if (d.type === 'call_ringing') {
            handleCallRinging(d);
        } else if (d.type === 'call_accepted') {
            handleCallAccepted(d);
        } else if (d.type === 'call_incoming') {
            handleCallIncoming(d);
        } else if (d.type === 'call_sdp') {
            handleCallSdp(d);
        } else if (d.type === 'call_ice') {
            handleCallIce(d);
        } else if (d.type === 'call_end') {
            handleCallEnd(d);
        } else if (d.type === 'call_rejected') {
            handleCallRejected(d);
        } else if (d.type === 'call_unavailable') {
            handleCallUnavailable(d);
        }
    }

