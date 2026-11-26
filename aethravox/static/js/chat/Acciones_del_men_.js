import { S } from './state.js';
import { escapeHtml, showToast } from './Utilidades.js';

    // ====== Acciones del menú ======

    // Copiar texto del mensaje
    export async function copyMessageText(el) {
        const bodyEl = el.querySelector('.chat-msg-body');
        if (!bodyEl) return;
        // Si es imagen, archivo o nota de voz, no se puede copiar texto
        if (bodyEl.querySelector('img, audio, .chat-file-card, .chat-voice-note')) {
            showToast('No se puede copiar un archivo multimedia', 'info'); return;
        }
        // Tomar solo el texto plano (sin el badge "editado")
        const clone = bodyEl.cloneNode(true);
        clone.querySelectorAll('.chat-msg-edited-badge').forEach(e => e.remove());
        const text = (clone.textContent || '').trim();
        if (!text) return;
        try {
            await navigator.clipboard.writeText(text);
            showToast('Texto copiado', 'success', 1500);
        } catch (err) {
            showToast('No se pudo copiar', 'warning');
        }
    }

    // Modal: info del mensaje
    export async function showMessageInfo(msgId) {
        try {
            const r = await fetch('/api/chat/message/' + msgId + '/info');
            if (!r.ok) {
                const err = await r.json().catch(() => ({}));
                showToast(err.error || 'Error al cargar info', 'error');
                return;
            }
            const info = await r.json();
            const statusText = info.deleted_at ? 'Eliminado' :
                info.status === 'read' ? 'Leído' :
                info.status === 'delivered' ? 'Entregado' :
                'Enviado';
            const bodyHtml = info.body
                ? `<div class="msg-info-body">${escapeHtml(info.body)}</div>`
                : `<div class="msg-info-body muted"><em>Contenido eliminado</em></div>`;
            const editedRow = info.edited_at
                ? `<div class="msg-info-row"><span>Editado:</span><strong>${escapeHtml(info.edited_at)}</strong></div>`
                : '';
            const deletedRow = info.deleted_at
                ? `<div class="msg-info-row danger"><span>Eliminado:</span><strong>${escapeHtml(info.deleted_at)}</strong></div>`
                : '';
            const overlay = document.createElement('div');
            overlay.className = 'modal-overlay modal-visible';
            overlay.innerHTML = `
                <div class="modal-dialog" role="dialog" aria-modal="true" onclick="event.stopPropagation()">
                    <div class="modal-header"><span>Info del mensaje</span><button type="button" class="modal-close">×</button></div>
                    <div class="modal-body">
                        ${bodyHtml}
                        <div class="msg-info-grid">
                            <div class="msg-info-row"><span>Enviado:</span><strong>${escapeHtml(info.created_at)}</strong></div>
                            <div class="msg-info-row"><span>Estado:</span><strong>${escapeHtml(statusText)}</strong></div>
                            ${editedRow}
                            ${deletedRow}
                        </div>
                    </div>
                    <div class="modal-actions"><button type="button" class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Cerrar</button></div>
                </div>`;
            overlay.addEventListener('click', () => overlay.remove());
            overlay.querySelector('.modal-close').addEventListener('click', () => overlay.remove());
            document.body.appendChild(overlay);
        } catch (e) {
            console.error('showMessageInfo:', e);
            showToast('Error de red', 'error');
        }
    }

    // Modal: editar mensaje (inline, sin abrir modal grande)
    export function startEditMessage(msgId, el) {
        const bodyEl = el.querySelector('.chat-msg-body');
        const timeEl = el.querySelector('.chat-msg-time');
        const menuBtn = el.querySelector('.chat-msg-menu-btn');
        if (!bodyEl) return;
        // Tomar texto original (sin badge editado)
        const clone = bodyEl.cloneNode(true);
        clone.querySelectorAll('.chat-msg-edited-badge').forEach(e => e.remove());
        const text = clone.querySelector('img, audio, .chat-file-card, .chat-voice-note') ? '' : (clone.textContent || '').trim();
        if (!text) { showToast('Solo se pueden editar mensajes de texto', 'info'); return; }

        const ta = document.createElement('textarea');
        ta.className = 'chat-edit-input';
        ta.value = text;
        ta.rows = 1;
        ta.maxLength = 4000;

        const saveBtn = document.createElement('button');
        saveBtn.className = 'btn btn-primary btn-sm';
        saveBtn.textContent = 'Guardar';
        const cancelBtn = document.createElement('button');
        cancelBtn.className = 'btn btn-secondary btn-sm';
        cancelBtn.textContent = 'Cancelar';

        const actions = document.createElement('div');
        actions.className = 'chat-edit-actions';
        actions.appendChild(saveBtn);
        actions.appendChild(cancelBtn);

        const wrap = document.createElement('div');
        wrap.className = 'chat-edit-wrap';
        wrap.appendChild(ta);
        wrap.appendChild(actions);

        // Reemplazar el body + time + menú por el editor
        const original = { body: bodyEl.innerHTML, time: timeEl.innerHTML };
        bodyEl.innerHTML = '';
        bodyEl.appendChild(wrap);
        if (timeEl) timeEl.style.display = 'none';
        if (menuBtn) menuBtn.style.display = 'none';

        ta.style.height = 'auto';
        ta.style.height = Math.min(ta.scrollHeight, 200) + 'px';
        ta.focus();
        ta.addEventListener('input', () => {
            ta.style.height = 'auto';
            ta.style.height = Math.min(ta.scrollHeight, 200) + 'px';
        });
        ta.addEventListener('keydown', (e) => {
            if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); saveBtn.click(); }
            if (e.key === 'Escape') { e.preventDefault(); cancelBtn.click(); }
        });

        cancelBtn.addEventListener('click', () => {
            bodyEl.innerHTML = original.body;
            if (timeEl) timeEl.style.display = '';
            if (menuBtn) menuBtn.style.display = '';
        });
        saveBtn.addEventListener('click', async () => {
            const newBody = ta.value.trim();
            if (!newBody) { showToast('El mensaje no puede estar vacío', 'warning'); return; }
            if (newBody === text) { cancelBtn.click(); return; }
            saveBtn.disabled = true;
            saveBtn.textContent = '…';
            try {
                const r = await fetch('/api/chat/message/' + msgId + '/edit', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
                    body: 'body=' + encodeURIComponent(newBody),
                });
                const d = await r.json();
                if (!r.ok) { showToast(d.error || 'Error al editar', 'error'); saveBtn.disabled = false; saveBtn.textContent = 'Guardar'; return; }
                // El WS broadcastará message_edited a todos. La actualización del DOM
                // la hace el handler del WS (tanto para el editor como para los demás).
                // Solo restauramos el layout por si el WS no llega.
                bodyEl.innerHTML = `<span>${escapeHtml(newBody)}</span><span class="chat-msg-edited-badge" title="Mensaje editado">editado</span>`;
                el.dataset.deleted = '0';
                if (timeEl) timeEl.style.display = '';
                if (menuBtn) menuBtn.style.display = '';
            } catch (err) {
                console.error('edit:', err);
                showToast('Error de red', 'error');
                saveBtn.disabled = false;
                saveBtn.textContent = 'Guardar';
            }
        });
    }

    // Modal custom (sin confirm() del navegador) para borrar
    export function confirmDeleteMessage(msgId) {
        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay modal-visible';
        overlay.innerHTML = `
            <div class="modal-dialog" role="dialog" aria-modal="true" onclick="event.stopPropagation()">
                <div class="modal-header"><span>Eliminar mensaje</span><button type="button" class="modal-close">×</button></div>
                <div class="modal-body">
                    <p>¿Seguro que quieres eliminar este mensaje? Esta acción no se puede deshacer.</p>
                </div>
                <div class="modal-actions">
                    <button type="button" class="btn btn-secondary" data-act="cancel">Cancelar</button>
                    <button type="button" class="btn btn-danger" data-act="delete">Eliminar</button>
                </div>
            </div>`;
        overlay.addEventListener('click', () => overlay.remove());
        overlay.querySelector('.modal-close').addEventListener('click', () => overlay.remove());
        overlay.querySelector('[data-act="cancel"]').addEventListener('click', () => overlay.remove());
        overlay.querySelector('[data-act="delete"]').addEventListener('click', async () => {
            const btn = overlay.querySelector('[data-act="delete"]');
            btn.disabled = true;
            btn.textContent = 'Eliminando…';
            try {
                const r = await fetch('/api/chat/message/' + msgId + '/delete', { method: 'POST' });
                const d = await r.json();
                if (!r.ok) { showToast(d.error || 'Error al eliminar', 'error'); btn.disabled = false; btn.textContent = 'Eliminar'; return; }
                overlay.remove();
                // El WS broadcastará message_deleted → el DOM se actualiza solo.
            } catch (e) {
                showToast('Error de red', 'error');
                btn.disabled = false;
                btn.textContent = 'Eliminar';
            }
        });
        document.body.appendChild(overlay);
    }
    window.confirmDeleteMessage = confirmDeleteMessage;

