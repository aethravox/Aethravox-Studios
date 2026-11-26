import { S } from './state.js';
import { selectConv } from './Seleccionar_conversaci_n.js';
import { loadConversations } from './Cargar_conversaciones.js';
import { closeNewChatModal } from './Modal__Nuevo_chat.js';
import { showToast } from './Utilidades.js';

    // ====== Crear grupo ======
    document.getElementById('new-group-form').addEventListener('submit', async (e) => {
        e.preventDefault();
        const name = document.getElementById('group-name-input').value.trim();
        if (!name) { showToast('Ponle un nombre al grupo', 'warning'); return; }
        const checked = document.querySelectorAll('#group-members-list input[name="members"]:checked');
        const members = Array.from(checked).map(c => c.value);
        if (members.length === 0) {
            showToast('Selecciona al menos un amigo', 'warning');
            return;
        }
        const description = document.getElementById('group-description-input').value.trim();
        try {
            let body = 'name=' + encodeURIComponent(name) + '&members=' + encodeURIComponent(members.join(','));
            if (description) body += '&description=' + encodeURIComponent(description);
            const r = await fetch('/api/chat/new_group', {
                method: 'POST',
                headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
                body: body,
            });
            if (!r.ok) {
                const err = await r.json().catch(() => ({}));
                showToast(err.error || 'Error al crear grupo', 'error');
                return;
            }
            const d = await r.json();
            closeNewChatModal();
            await loadConversations();
            if (d.id) selectConv(d.id);
            showToast('Grupo creado', 'success', 2000);
        } catch (e) {
            console.error('createGroup:', e);
            showToast('Error de red', 'error');
        }
    });

