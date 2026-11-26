(function() {
    'use strict';

    function openModal(id) {
        const modal = document.getElementById(id);
        modal.hidden = false;
        requestAnimationFrame(() => modal.classList.add('modal-visible'));
    }
    function closeModal(id) {
        const modal = document.getElementById(id);
        modal.classList.remove('modal-visible');
        setTimeout(() => { modal.hidden = true; }, 200);
    }

    window.openSuspendModal = function(userId, username) {
        document.getElementById('suspend-username').textContent = username;
        document.getElementById('suspend-form').action = '/admin/users/' + userId + '/suspend';
        openModal('suspend-modal');
    };
    window.closeSuspendModal = function() { closeModal('suspend-modal'); };

    window.openBanModal = function(userId, username) {
        document.getElementById('ban-username').textContent = username;
        document.getElementById('ban-form').action = '/admin/users/' + userId + '/ban';
        // Reset form: permanent por defecto
        document.getElementById('ban-permanent-radio').checked = true;
        document.getElementById('ban-permanent-field').value = '1';
        document.getElementById('ban-duration-fields').style.display = 'none';
        openModal('ban-modal');
    };
    window.closeBanModal = function() { closeModal('ban-modal'); };

    // Toggle del radio "permanente vs temporal" en ban
    document.querySelectorAll('input[name="ban_type"]').forEach(radio => {
        radio.addEventListener('change', function() {
            const isPermanent = document.getElementById('ban-permanent-radio').checked;
            document.getElementById('ban-permanent-field').value = isPermanent ? '1' : '';
            document.getElementById('ban-duration-fields').style.display = isPermanent ? 'none' : 'block';
        });
    });

    // Cerrar modales con Esc o click fuera
    document.addEventListener('keydown', e => {
        if (e.key === 'Escape') {
            closeSuspendModal();
            closeBanModal();
        }
    });
    document.getElementById('suspend-modal').addEventListener('click', e => {
        if (e.target.id === 'suspend-modal') closeSuspendModal();
    });
    document.getElementById('ban-modal').addEventListener('click', e => {
        if (e.target.id === 'ban-modal') closeBanModal();
    });

    // ===== Modal: Historial de IPs de un usuario =====
    window.openIpHistoryModal = async function(userId, username) {
        const modal = document.getElementById('ip-history-modal');
        document.getElementById('ip-history-username').textContent = username;
        document.getElementById('ip-history-list').innerHTML = '<p class="muted">Cargando…</p>';
        modal.hidden = false;
        requestAnimationFrame(() => modal.classList.add('modal-visible'));

        try {
            const resp = await fetch('/admin/users/' + userId + '/ips');
            if (!resp.ok) {
                document.getElementById('ip-history-list').innerHTML = '<p class="muted">Error al cargar.</p>';
                return;
            }
            const list = await resp.json();
            if (list.length === 0) {
                document.getElementById('ip-history-list').innerHTML = '<p class="muted">Sin logins registrados todavía.</p>';
                return;
            }
            document.getElementById('ip-history-list').innerHTML = list.map(item => `
                <div class="ip-row${item.is_first_time ? ' ip-row-new' : ''}">
                    <div class="ip-row-icon">${item.is_first_time ? '⚠️' : '📍'}</div>
                    <div class="ip-row-info">
                        <div class="ip-row-ip">${escapeHtml(item.ip)}</div>
                        <div class="ip-row-meta muted">
                            Primera vez: ${item.first_seen} · Última vez: ${item.last_seen} · Veces: ${item.times_used}
                            ${item.is_first_time ? '<span class="ip-new-badge">NUEVA IP</span>' : ''}
                        </div>
                    </div>
                </div>
            `).join('');
        } catch (e) {
            document.getElementById('ip-history-list').innerHTML = '<p class="muted">Error de red.</p>';
        }
    };
    window.closeIpHistoryModal = function() {
        const modal = document.getElementById('ip-history-modal');
        modal.classList.remove('modal-visible');
        setTimeout(() => { modal.hidden = true; }, 200);
    };

    function escapeHtml(s) {
        return String(s)
            .replace(/&/g, '&amp;').replace(/</g, '&lt;')
            .replace(/>/g, '&gt;').replace(/"/g, '&quot;');
    }

    document.addEventListener('keydown', e => {
        if (e.key === 'Escape') closeIpHistoryModal();
    });
    document.getElementById('ip-history-modal').addEventListener('click', e => {
        if (e.target.id === 'ip-history-modal') closeIpHistoryModal();
    });
})();
