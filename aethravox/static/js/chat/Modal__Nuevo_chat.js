import { S } from './state.js';
import { escapeHtml, showToast } from './Utilidades.js';
import { loadConversations } from './Cargar_conversaciones.js';
import { selectConv } from './Seleccionar_conversaci_n.js';

    // ====== Modal: Nuevo chat ======
    export async function loadFriends() {
        if (S.cachedFriends) return S.cachedFriends;
        try {
            const r = await fetch('/api/friends/list');
            if (!r.ok) { S.cachedFriends = []; return []; }
            S.cachedFriends = await r.json();
            return S.cachedFriends;
        } catch (e) { return []; }
    }

    export async function openNewChatModal(tab) {
        const modal = document.getElementById('new-chat-modal');
        modal.hidden = false;
        requestAnimationFrame(() => modal.classList.add('modal-visible'));
        switchTab(tab || 'dm');

        // Cargar amigos si no están cacheados
        if (!S.cachedFriends) {
            await loadFriends();
        }
        renderFriendsList();
        renderGroupMembersList();
    }
    window.openNewChatModal = openNewChatModal;

    export function closeNewChatModal() {
        const modal = document.getElementById('new-chat-modal');
        modal.classList.remove('modal-visible');
        setTimeout(() => { modal.hidden = true; }, 200);
    }
    window.closeNewChatModal = closeNewChatModal;

    export function switchTab(tab) {
        document.querySelectorAll('.chat-new-tab').forEach(t => t.classList.remove('active'));
        const btn = document.querySelector(`.chat-new-tab[data-tab="${tab}"]`);
        if (btn) btn.classList.add('active');
        document.getElementById('new-dm-panel').hidden = (tab !== 'dm');
        document.getElementById('new-group-panel').hidden = (tab !== 'group');
    }
    window.switchTab = switchTab;

    export function renderFriendsList() {
        const container = document.getElementById('friends-list');
        const noFriends = document.getElementById('no-friends-state');
        const friendsContainer = document.getElementById('friends-list-container');

        if (!S.cachedFriends || S.cachedFriends.length === 0) {
            friendsContainer.hidden = true;
            noFriends.hidden = false;
            return;
        }
        noFriends.hidden = true;
        friendsContainer.hidden = false;
        container.innerHTML = S.cachedFriends.map(f => `
            <div class="friend-pick" onclick="startDM(${f.id}, '${escapeHtml(f.username)}')">
                <div class="friend-pick-avatar-wrap">
                    <img src="${escapeHtml(f.avatar)}" alt="avatar" class="friend-pick-avatar" width="40" height="40">
                    ${f.online ? '<span class="friend-online-dot" title="En línea"></span>' : ''}
                </div>
                <div class="friend-pick-info">
                    <div class="friend-pick-name">${escapeHtml(f.display)}</div>
                    <div class="friend-pick-username muted">@${escapeHtml(f.username)}${f.online ? ' · <span class="status-active">en línea</span>' : ''}</div>
                </div>
                <button type="button" class="btn btn-primary btn-sm">💬 Chatear</button>
            </div>
        `).join('');
    }

    export async function startDM(friendId, username) {
        try {
            const r = await fetch('/api/chat/new_dm', {
                method: 'POST',
                headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
                body: 'username=' + encodeURIComponent(username),
            });
            if (!r.ok) {
                const err = await r.json().catch(() => ({}));
                showToast(err.error || 'Error al crear el chat', 'error');
                return;
            }
            const d = await r.json();
            closeNewChatModal();
            await loadConversations();
            if (d.id) selectConv(d.id);
            showToast('Chat abierto con @' + username, 'success', 2000);
        } catch (e) {
            console.error('startDM:', e);
            showToast('Error de red', 'error');
        }
    }
    window.startDM = startDM;

    export function renderGroupMembersList() {
        const container = document.getElementById('group-members-list');
        if (!S.cachedFriends || S.cachedFriends.length === 0) {
            container.innerHTML = '<p class="muted">No tienes amigos para añadir al grupo. Agrega algunos primero desde <a href="/friends">/friends</a>.</p>';
            return;
        }
        container.innerHTML = S.cachedFriends.map(f => `
            <label class="group-member-pick">
                <input type="checkbox" name="members" value="${escapeHtml(f.username)}">
                <div class="group-member-info">
                    <img src="${escapeHtml(f.avatar)}" alt="avatar" class="group-member-avatar" width="32" height="32">
                    <div>
                        <div class="friend-pick-name">${escapeHtml(f.display)}</div>
                        <div class="friend-pick-username muted">@${escapeHtml(f.username)}</div>
                    </div>
                </div>
            </label>
        `).join('');
    }

