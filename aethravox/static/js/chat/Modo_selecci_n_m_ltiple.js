import { S } from './state.js';
import { showToast } from './Utilidades.js';

    // ====== Modo selección múltiple de mensajes ======
    // Estado: si está activo, qué mensajes están seleccionados (Set de IDs).
    // Se activa desde el menú ⋯ de un mensaje (PC: opción "Seleccionar")
    // o manteniendo presionado un mensaje (móvil: long-press).


    export function enterSelectMode() {
        if (!S.currentConv) { showToast('Abre una conversación primero', 'info'); return; }
        S.selectMode = true;
        // NO limpiar selectedIds aquí: si venimos del menú ⋯, ya pre-seleccionamos
        // un mensaje antes de llamar a enterSelectMode().
        document.getElementById('chat-messages').classList.add('chat-select-mode');
        S.selectBar.hidden = false;
        updateSelectCount();
        bindSelectClicks();
    }
    export function exitSelectMode() {
        S.selectMode = false;
        S.selectedIds.clear();
        document.getElementById('chat-messages').classList.remove('chat-select-mode');
        S.selectBar.hidden = true;
        // Limpiar clases is-selected de todos los mensajes
        document.querySelectorAll('.chat-msg.is-selected').forEach(el => el.classList.remove('is-selected'));
    }
    export function updateSelectCount() {
        const n = S.selectedIds.size;
        S.selectCountEl.textContent = n === 1 ? '1 seleccionado' : (n + ' seleccionados');
        S.selectDeleteBtn.disabled = n === 0;
        S.selectDeleteBtn.textContent = n === 0 ? '🗑 Borrar' : `🗑 Borrar (${n})`;
    }
    // Marcar/desmarcar un mensaje individual (lo usa bindSelectClicks y el long-press).
    export function toggleSelectMessage(el) {
        if (el.dataset.deleted === '1') return;
        const id = parseInt(el.dataset.id, 10);
        if (S.selectedIds.has(id)) {
            S.selectedIds.delete(id);
            el.classList.remove('is-selected');
        } else {
            S.selectedIds.add(id);
            el.classList.add('is-selected');
        }
        updateSelectCount();
    }
    // Asignar click de selección a cada mensaje en modo selección.
    export function bindSelectClicks() {
        document.querySelectorAll('.chat-messages.chat-select-mode .chat-msg').forEach(el => {
            // Evitar doble-bind: marcamos con data-bound.
            if (el.dataset.selectBound === '1') return;
            el.dataset.selectBound = '1';
            el.addEventListener('click', function onSelectClick(e) {
                if (!S.selectMode) return;
                e.stopPropagation();
                toggleSelectMessage(el);
            });
        });
    }
    // Botones de la barra flotante.
    S.selectCancelBtn.addEventListener('click', exitSelectMode);
    S.selectAllBtn.addEventListener('click', () => {
        // Seleccionar todos los mensajes visibles NO borrados.
        document.querySelectorAll('.chat-messages.chat-select-mode .chat-msg').forEach(el => {
            if (el.dataset.deleted === '1') return;
            const id = parseInt(el.dataset.id, 10);
            S.selectedIds.add(id);
            el.classList.add('is-selected');
        });
        updateSelectCount();
    });
    S.selectDeleteBtn.addEventListener('click', async () => {
        if (S.selectedIds.size === 0) return;
        const ids = [...selectedIds];
        const n = ids.length;
        // Confirmar antes de borrar (modal custom, no confirm() del navegador).
        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay modal-visible';
        overlay.innerHTML = `
            <div class="modal-dialog" role="dialog" aria-modal="true" onclick="event.stopPropagation()">
                <div class="modal-header"><span>Eliminar ${n} mensajes</span><button type="button" class="modal-close">×</button></div>
                <div class="modal-body">
                    <p>¿Seguro que quieres eliminar <strong>${n}</strong> mensaje${n === 1 ? '' : 's'}? Esta acción no se puede deshacer.</p>
                    <p class="muted small">Solo se borrarán los mensajes en los que tengas permiso (tuyos dentro de 30 min, o ajenos si eres admin del grupo con permiso de borrado). Los que no tengas permiso para borrar se quedan.</p>
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
                const r = await fetch('/api/chat/messages/bulk-delete', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ ids: ids }),
                });
                const d = await r.json();
                if (!r.ok) {
                    showToast(d.error || 'Error al eliminar', 'error');
                    btn.disabled = false; btn.textContent = 'Eliminar';
                    return;
                }
                const okCount = d.ok_count || 0;
                const deniedCount = d.denied_count || 0;
                overlay.remove();
                exitSelectMode();
                // El WS broadcastea message_deleted por cada uno → el DOM se
                // actualiza solo. Pero avisamos del resultado.
                if (deniedCount === 0) {
                    showToast(`${okCount} mensaje${okCount === 1 ? '' : 's'} eliminado${okCount === 1 ? '' : 's'}`, 'success');
                } else if (okCount === 0) {
                    showToast('No se pudo borrar ningún mensaje (¿sin permiso o pasaron 30 min?)', 'warning');
                } else {
                    showToast(`${okCount} borrados · ${deniedCount} sin permiso`, 'warning');
                }
            } catch (e) {
                showToast('Error de red', 'error');
                btn.disabled = false; btn.textContent = 'Eliminar';
            }
        });
        document.body.appendChild(overlay);
    });
    window.exitSelectMode = exitSelectMode;

