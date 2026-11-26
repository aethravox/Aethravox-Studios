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

    // ===== Modal: Añadir sección =====
    window.openCreateWebModal = function() {
        // Reset form
        document.getElementById('type-external').checked = true;
        document.getElementById('external-panel').style.display = 'block';
        document.getElementById('custom-panel').style.display = 'none';
        document.getElementById('create-web-form').reset();
        openModal('create-web-modal');
    };
    window.closeCreateWebModal = function() { closeModal('create-web-modal'); };

    // Toggle tipo de sección (external vs custom)
    document.querySelectorAll('input[name="url_type"]').forEach(radio => {
        radio.addEventListener('change', function() {
            const isCustom = document.getElementById('type-custom').checked;
            document.getElementById('external-panel').style.display = isCustom ? 'none' : 'block';
            document.getElementById('custom-panel').style.display = isCustom ? 'block' : 'none';
            // Si es custom, vaciar el url field porque se ignora
            if (isCustom) {
                document.querySelector('input[name="url"]').value = '';
            }
        });
    });

    // ===== Modal: Editar HTML =====
    window.openEditHtmlModal = function(webId, name) {
        document.getElementById('edit-html-name').textContent = name;
        document.getElementById('edit-html-form').action = '/admin/webs/' + webId + '/html';
        // Cargar HTML actual via API (no crítica, si falla solo textarea vacía)
        // TODO: añadir endpoint GET /api/admin/webs/:id/html para esto
        // Por ahora el admin puede pegar su HTML nuevo
        document.getElementById('edit-html-textarea').value = '';
        openModal('edit-html-modal');
    };
    window.closeEditHtmlModal = function() { closeModal('edit-html-modal'); };

    // ===== Modal: Eliminar =====
    window.openDeleteWebModal = function(webId, name, slug) {
        document.getElementById('delete-web-name').textContent = name;
        document.getElementById('delete-web-slug').textContent = '/' + slug;
        document.getElementById('delete-web-form').action = '/admin/webs/' + webId + '/delete';
        openModal('delete-web-modal');
    };
    window.closeDeleteWebModal = function() { closeModal('delete-web-modal'); };

    // ===== Modal: Desactivar (con confirmación especial según tipo) =====
    // isSystem: true para FEATURE (chat, friends, docs, playground)
    // isCustom: true para CUSTOM (HTML propio o URL externa)
    // hasHtml: true si tiene HTML custom (se sirve en /slug)
    window.openDeactivateModal = function(webId, name, isSystem, isCustom, hasHtml) {
        document.getElementById('deactivate-web-name').textContent = name;
        document.getElementById('deactivate-web-form').action = '/admin/webs/' + webId + '/toggle';

        // Mostrar warning según tipo:
        // - FEATURE → warning-system
        // - CUSTOM → warning-custom (+ sub-warning según si tiene HTML o es URL externa)
        document.getElementById('deactivate-warning-system').hidden = !isSystem;
        document.getElementById('deactivate-warning-custom').hidden = !isCustom;
        document.getElementById('deactivate-custom-html-warning').hidden = !(isCustom && hasHtml);
        document.getElementById('deactivate-custom-external-warning').hidden = !(isCustom && !hasHtml);

        openModal('deactivate-web-modal');
    };
    window.closeDeactivateWebModal = function() { closeModal('deactivate-web-modal'); };

    // ===== Modal: Permisos =====
    window.openPermsModal = function(webId, name, hasOwner, hasAdmin, hasStaff, hasUsuario) {
        document.getElementById('perms-web-name').textContent = name;
        document.getElementById('perms-web-form').action = '/admin/webs/' + webId + '/perms';
        document.getElementById('perm-owner').checked = hasOwner;
        document.getElementById('perm-admin').checked = hasAdmin;
        document.getElementById('perm-staff').checked = hasStaff;
        document.getElementById('perm-usuario').checked = hasUsuario;
        openModal('perms-web-modal');
    };
    window.closePermsModal = function() { closeModal('perms-web-modal'); };

    // ===== Cerrar con Esc =====
    document.addEventListener('keydown', e => {
        if (e.key === 'Escape') {
            closeCreateWebModal();
            closeEditHtmlModal();
            closeDeleteWebModal();
            closeDeactivateWebModal();
            closePermsModal();
        }
    });

    // ===== Click fuera cierra =====
    ['create-web-modal', 'edit-html-modal', 'delete-web-modal', 'deactivate-web-modal', 'perms-web-modal'].forEach(id => {
        const m = document.getElementById(id);
        if (m) m.addEventListener('click', e => {
            if (e.target === m) closeModal(id);
        });
    });
})();
