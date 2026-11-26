import { S } from './state.js';
import { renderMessages } from './Men__contextual_del_mensaje.js';
import { escapeHtml } from './Utilidades.js';
import { showCallButtons } from './Llamadas___videollamadas.js';
import { initialAvatar, renderSidebar } from './Cargar_conversaciones.js';

    // ====== Seleccionar conversación ======
    export async function selectConv(id) {
        S.currentConv = id;
        // NO guardamos en localStorage: si lo hiciéramos, al recargar /chat se
        // reabriría el último chat en vez de quedar en la lista de recientes.
        // (Este bug se había "arreglado" y roto ~10 veces porque cada vez que
        //  alguien reintroducía el setItem, volvía a pasar.)
        const inputBar = document.getElementById('chat-input-bar');
        inputBar.style.display = 'flex';
        // Salir del modo selección múltiple si estábamos en él (la conv cambia).
        if (window.exitSelectMode) window.exitSelectMode();
        // Mostrar el botón ☑ de selección múltiple (solo tiene sentido con conv abierta).
        const selectToggle = document.getElementById('chat-select-toggle');
        if (selectToggle) selectToggle.hidden = false;

        document.getElementById('chat-app').classList.add('chat-open');

        try {
            const r = await fetch('/api/chat/messages/' + id, { cache: 'no-store' });
            const msgs = await r.json();
            // Buscar info de la conversación en DMs/grupos sueltos Y en grupos de comunidad
            let conv = S.conversations.find(x => x.id === id);
            if (!conv) {
                for (const c of S.communities) {
                    const g = (c.groups || []).find(x => x.id === id);
                    if (g) {
                        conv = { id: g.id, conv_type: 'group', name: g.name,
                                 description: g.description, image_url: null,
                                 last_message: '', last_message_at: '', unread_count: 0 };
                        // Lo añadimos a conversations para futuras búsquedas
                        if (!S.conversations.find(x => x.id === g.id)) S.conversations.push(conv);
                        break;
                    }
                }
            }
            const headerInfo = document.getElementById('chat-header-info');
            if (conv) {
                const isGroup = conv.conv_type === 'group';
                let headerAvatar;
                if (conv.image_url) {
                    headerAvatar = `<img src="${escapeHtml(conv.image_url)}" alt="" class="chat-header-avatar" style="width:32px;height:32px;border-radius:50%;object-fit:cover;">`;
                } else if (isGroup) {
                    headerAvatar = `<span class="chat-header-avatar">👥</span>`;
                } else {
                    headerAvatar = `<img src="${initialAvatar(conv.name, 32)}" alt="" class="chat-header-avatar" style="width:32px;height:32px;border-radius:50%;object-fit:cover;">`;
                }
                const descHtml = (isGroup && conv.description)
                    ? `<span class="chat-header-desc muted small">${escapeHtml(conv.description)}</span>`
                    : '';
                headerInfo.innerHTML = `${headerAvatar}<span class="chat-header-info-text"><span class="chat-header-name">${escapeHtml(conv.name)}</span>${descHtml}</span><span class="chat-typing-indicator" id="typing-indicator"></span>`;
                // Mostrar los botones de llamada solo en DMs (las llamadas son 1:1).
                showCallButtons(!isGroup);
            }
            renderMessages(msgs);
            // Limpiar badge inmediatamente (DMs y grupos de comunidad)
            S.conversations.forEach(c => { if (c.id === id) c.unread_count = 0; });
            S.communities.forEach(c => (c.groups || []).forEach(g => { if (g.id === id) g.unread = 0; }));
            renderSidebar();
            fetch('/api/chat/read/' + id, { method: 'POST' });
        } catch (e) {
            console.error('selectConv:', e);
        }
    }
    window.selectConv = selectConv;

