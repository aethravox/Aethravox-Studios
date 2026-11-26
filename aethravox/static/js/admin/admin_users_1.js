// ===== Toggle de cada fila de usuario (mostrar/ocultar detalle) =====
function toggleUserRow(btn) {
    const li = btn.closest('.user-list-item');
    const detail = li.querySelector('.user-list-detail');
    const expanded = btn.getAttribute('aria-expanded') === 'true';
    btn.setAttribute('aria-expanded', expanded ? 'false' : 'true');
    detail.hidden = expanded;
}
// ===== Buscador por nombre =====
(function() {
    const input = document.getElementById('user-search');
    const list = document.getElementById('user-list');
    const empty = document.getElementById('user-list-empty');
    if (!input || !list) return;
    input.addEventListener('input', function() {
        const q = input.value.trim().toLowerCase();
        let visible = 0;
        list.querySelectorAll('.user-list-item').forEach(li => {
            const name = (li.dataset.name || '').toLowerCase();
            const match = !q || name.includes(q);
            li.style.display = match ? '' : 'none';
            if (match) visible++;
        });
        if (empty) empty.hidden = visible !== 0;
    });
})();
