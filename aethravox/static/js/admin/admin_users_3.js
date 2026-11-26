(function() {
    // Re-usar el IIFE existente del admin_users no es trivial — declarar en el global scope.
    function openModal(id) {
        const m = document.getElementById(id);
        m.hidden = false;
        requestAnimationFrame(() => m.classList.add('modal-visible'));
    }
    function closeModal(id) {
        const m = document.getElementById(id);
        m.classList.remove('modal-visible');
        setTimeout(() => { m.hidden = true; }, 200);
    }
    window.openDeleteUserModal = function(userId, username) {
        document.getElementById('delete-user-name').textContent = username;
        document.getElementById('delete-user-form').action = '/admin/users/' + userId + '/delete';
        openModal('delete-user-modal');
    };
    window.closeDeleteUserModal = function() { closeModal('delete-user-modal'); };
    document.addEventListener('keydown', e => {
        if (e.key === 'Escape') closeDeleteUserModal();
    });
    document.getElementById('delete-user-modal').addEventListener('click', e => {
        if (e.target.id === 'delete-user-modal') closeDeleteUserModal();
    });
})();
