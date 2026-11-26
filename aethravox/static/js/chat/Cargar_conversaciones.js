import { S } from './state.js';
import { escapeHtml } from './Utilidades.js';

    // ====== Cargar conversaciones + comunidades ======
    export async function loadConversations() {
        try {
            const [rConv, rComm] = await Promise.all([
                fetch('/api/chat/conversations', { cache: 'no-store' }),
                fetch('/api/chat/communities', { cache: 'no-store' }),
            ]);
            if (rConv.ok) S.conversations = await rConv.json();
            if (rComm.ok) S.communities = await rComm.json();
            renderSidebar();
        } catch (e) { console.error('loadConversations:', e); }
    }

    export function renderSidebar() {
        const list = document.getElementById('chat-list');
        // Conjunto de IDs de grupos que pertenecen a una comunidad. Los sacamos
        // de la lista plana de "Mensajes directos" para que SOLO aparezcan dentro
        // de su comunidad (en el árbol de abajo). Sin esto, un grupo de comunidad
        // se duplicaba: salía en directos Y bajo su comunidad, y al hablar en él
        // "subía" arriba en directos aunque perteneciera a una comunidad.
        const communityGroupIds = new Set();
        (S.communities || []).forEach(c => (c.groups || []).forEach(g => communityGroupIds.add(g.id)));
        const directConvs = (S.conversations || []).filter(c => !communityGroupIds.has(c.id));

        if (directConvs.length === 0 && (!S.communities || S.communities.length === 0)) {
            list.innerHTML = '<div class="chat-empty-mini muted">No tienes conversaciones todavía. Crea una con <strong>+</strong>.</div>';
            return;
        }
        // 1. DMs y grupos sueltos (sección "Mensajes directos")
        let html = '';
        if (directConvs.length > 0) {
            html += '<div class="chat-list-section-title">💬 Mensajes directos</div>';
            html += directConvs.map(c => renderConvItem(c)).join('');
        }
        // 2. Comunidades con sus grupos (árbol estilo Discord).
        //    La comunidad muestra el TOTAL de no leídos de todos sus grupos;
        //    cada grupo muestra SOLO su propio número (ver renderCommunity).
        if (S.communities && S.communities.length > 0) {
            html += '<div class="chat-list-section-title" style="margin-top:14px">🏘️ Comunidades</div>';
            html += S.communities.map(comm => renderCommunity(comm)).join('');
        }
        list.innerHTML = html;
    }

    // Genera un SVG data-URI con la inicial del nombre, igual que el backend
    // (User.avatar() cuando no hay foto subida). Se usa como fallback cuando un
    // DM no trae image_url, en vez del emoji 👤 genérico.
    // El color de fondo varía según el nombre (hash → matiz HSL), así cada
    // usuario tiene un color distinto y no se ven todos iguales en negro.
    export function initialAvatar(name, size = 40) {
        const initial = (name || '?').trim().charAt(0).toUpperCase() || '?';
        const escaped = encodeURIComponent(initial);
        // Hash simple del nombre → 0..360 (matiz)
        let h = 0;
        for (let i = 0; i < (name || '').length; i++) {
            h = (h * 31 + name.charCodeAt(i)) % 360;
        }
        const hue = h;
        // Fondo oscuro con tinte del matiz, letra clara. SVG acepta hsl() sin '#'.
        // En data-URI escapamos comas y paréntesis con %2C y %28/%29.
        const bg = `hsl(${hue}%2C%2030%25%2C%2018%25)`;
        const fg = `hsl(${hue}%2C%2060%25%2C%2085%25)`;
        return `data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' width='${size}' height='${size}'><rect width='${size}' height='${size}' fill='${bg}'/><text x='${size/2}' y='${Math.round(size*0.63)}' font-size='${Math.round(size*0.5)}' fill='${fg}' text-anchor='middle' font-family='sans-serif'>${escaped}</text></svg>`;
    }

    export function renderConvItem(c) {
        const isGroup = c.conv_type === 'group';
        // Para DMs: si hay image_url (foto real o SVG del backend) la usamos;
        // si no, generamos el SVG con la inicial aquí (mejor que el emoji 👤).
        // Para grupos: 👥 (no hay avatar individual) o el image_url del grupo.
        let avatarHtml;
        if (c.image_url) {
            avatarHtml = `<img src="${escapeHtml(c.image_url)}" alt="" class="chat-list-avatar" style="width:40px;height:40px;border-radius:50%;object-fit:cover;">`;
        } else if (isGroup) {
            avatarHtml = '<div class="chat-list-avatar chat-list-avatar-group">👥</div>';
        } else {
            avatarHtml = `<img src="${initialAvatar(c.name, 40)}" alt="" class="chat-list-avatar" style="width:40px;height:40px;border-radius:50%;object-fit:cover;">`;
        }
        const active = S.currentConv === c.id ? ' active' : '';
        const lastRaw = c.last_message || 'Sin mensajes aún';
        // Resumen legible según el prefijo del body.
        const lastDisplay = lastRaw.startsWith('[IMG]') ? '📷 Imagen'
            : lastRaw.startsWith('[VOICE]') ? '🎙️ Nota de voz'
            : lastRaw.startsWith('[FILE]') ? (() => {
                const parts = lastRaw.substring(6).split(':');
                const orig = parts.length >= 3 ? parts.slice(1, parts.length - 2).join(':') : '';
                return '📎 ' + (orig || 'Archivo');
            })()
            : escapeHtml(lastRaw);
        const badge = c.unread_count > 0 ? `<span class="chat-list-badge">${c.unread_count}</span>` : '';
        return `
            <div class="chat-list-item${active}" data-id="${c.id}" onclick="selectConv(${c.id})">
                ${avatarHtml}
                <div class="chat-list-info">
                    <div class="chat-list-name">${escapeHtml(c.name)}</div>
                    <div class="chat-list-last">${lastDisplay}</div>
                </div>
                ${badge}
                <div class="chat-list-time">${escapeHtml(c.last_message_at || '')}</div>
            </div>
        `;
    }

    export function renderCommunity(comm) {
        const totalUnread = (comm.groups || []).reduce((s, g) => s + (g.unread || 0), 0);
        const commBadge = totalUnread > 0 ? `<span class="chat-list-badge">${totalUnread}</span>` : '';
        const groupId = 'comm-tree-' + comm.id;
        // ¿Algún grupo de esta comunidad está activo? Si sí, dejamos el árbol abierto.
        const hasActiveGroup = (comm.groups || []).some(g => g.id === S.currentConv);
        const openAttr = hasActiveGroup ? ' open' : '';
        const groupsHtml = (comm.groups || []).map(g => {
            const active = S.currentConv === g.id ? ' active' : '';
            const badge = g.unread > 0 ? `<span class="chat-list-badge">${g.unread}</span>` : '';
            const name = g.name || 'Grupo';
            return `
                <div class="chat-list-item chat-list-channel${active}" data-id="${g.id}" onclick="selectConv(${g.id})">
                    <span class="chat-channel-hash">#</span>
                    <div class="chat-list-info">
                        <div class="chat-list-name">${escapeHtml(name)}</div>
                        ${g.description ? `<div class="chat-list-last">${escapeHtml(g.description)}</div>` : ''}
                    </div>
                    ${badge}
                </div>
            `;
        }).join('');
        const emptyGroups = !comm.groups || comm.groups.length === 0
            ? '<div class="chat-list-channel-empty muted small">Sin grupos aún</div>' : '';
        return `
            <details class="chat-comm-tree"${openAttr}>
                <summary class="chat-comm-summary">
                    <span class="chat-comm-icon">${comm.icon || '🏘️'}</span>
                    <span class="chat-comm-name">${escapeHtml(comm.name)}</span>
                    ${commBadge}
                </summary>
                <div class="chat-comm-groups">
                    ${groupsHtml}
                    ${emptyGroups}
                    <a href="/community/${comm.id}" class="chat-comm-manage" title="Ver comunidad">⚙️ Gestionar comunidad</a>
                </div>
            </details>
        `;
    }

