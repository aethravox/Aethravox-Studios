(function() {
    'use strict';

    // ===== Modal logout-all =====
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
    window.openLogoutAllModal = function() { openModal('logout-all-modal'); };
    window.closeLogoutAllModal = function() { closeModal('logout-all-modal'); };
    document.addEventListener('keydown', e => {
        if (e.key === 'Escape') closeLogoutAllModal();
    });
    const logoutAllModal = document.getElementById('logout-all-modal');
    if (logoutAllModal) {
        logoutAllModal.addEventListener('click', e => {
            if (e.target.id === 'logout-all-modal') closeLogoutAllModal();
        });
    }

    // ===== Tabs de ajustes =====
    const tabs = document.querySelectorAll('.settings-tab');
    const sections = document.querySelectorAll('.settings-section');
    tabs.forEach(tab => {
        tab.addEventListener('click', () => {
            const target = tab.dataset.tab;
            tabs.forEach(t => t.classList.toggle('is-active', t === tab));
            sections.forEach(s => {
                const active = s.dataset.section === target;
                s.classList.toggle('is-active', active);
                s.hidden = !active;
            });
            // Persiste la pestaña activa en la URL (hash) para que sobreviva recargas.
            if (history.replaceState) history.replaceState(null, '', '#ajustes-' + target);
        });
    });
    // Restaura la pestaña desde el hash (#ajustes-tema, #tema) o query ?tab=tema.
    const urlParams = new URLSearchParams(location.search);
    const tabParam = urlParams.get('tab');
    const hash = location.hash.replace(/^#(?:ajustes-)?/, '');
    const initialTab = tabParam || hash;
    if (initialTab) {
        const initial = document.querySelector('.settings-tab[data-tab="' + initialTab + '"]');
        if (initial) initial.click();
    }

    // ===== Notificaciones: estado + botón de activación =====
    // Refresca el texto de estado y habilita/deshabilita el botón según el
    // permiso actual. El helper showDesktopNotification y el flag aethravoxNotifSupported
    // los define base.html (corren en todas las páginas autenticadas).
    function refreshNotifStatus() {
        const valueEl = document.getElementById('notif-status-value');
        const btn = document.getElementById('notif-enable-btn');
        if (!valueEl || !btn) return;
        if (!('Notification' in window)) {
            valueEl.textContent = 'No soportado en este navegador';
            valueEl.className = 'notif-status-value notif-status-unsupported';
            btn.disabled = true;
            btn.textContent = '🚫 No soportado';
            return;
        }
        const perm = Notification.permission;
        const labels = {
            'granted':  '✓ Activadas — recibirás avisos de mensajes y llamadas',
            'denied':   '✗ Bloqueadas — actívalas desde el icono de candado del navegador',
            'default':  '⏳ Pendiente — pulsa "Activar notificaciones" para pedir permiso',
        };
        valueEl.textContent = labels[perm] || perm;
        valueEl.className = 'notif-status-value notif-status-' + perm;
        if (perm === 'granted') {
            btn.disabled = true;
            btn.textContent = '✓ Ya activadas';
        } else if (perm === 'denied') {
            btn.disabled = true;
            btn.textContent = '🚫 Bloqueado (ver candado del navegador)';
        } else {  // default
            btn.disabled = false;
            btn.textContent = '🔔 Activar notificaciones';
        }
    }
    const notifBtn = document.getElementById('notif-enable-btn');
    if (notifBtn) {
        notifBtn.addEventListener('click', function() {
            if (!('Notification' in window)) return;
            // Al pedir desde aquí hay gesture de usuario garantizado.
            Notification.requestPermission().then(function(p) {
                refreshNotifStatus();
                if (p === 'granted' && typeof showToast === 'function') {
                    showToast('🔔 Notificaciones activadas. Te avisaremos de mensajes y llamadas.', 'success');
                }
            }).catch(function() { refreshNotifStatus(); });
        });
        refreshNotifStatus();
        // Si el permiso cambia en otra pestaña, refrescar al volver a esta.
        document.addEventListener('visibilitychange', function() {
            if (!document.hidden) refreshNotifStatus();
        });
    }
})();

// ===== Cropper circular de avatar (tipo WhatsApp) =====
// Flujo: input file → FileReader → imagen en memoria → modal con canvas +
// máscara circular → drag para mover, slider para zoom → al confirmar, se
// dibuja el recorte cuadrado (1:1, lo que se ve dentro del círculo) en un
// canvas oculto y se intercepta el submit del form para enviar un FormData
// con un Blob PNG real (campo "avatar"), que el backend upload_avatar recibe
// como multipart normal. Sin esto, una data URL en un input hidden iría como
// texto y el backend no la parsearía como imagen.
(function() {
    'use strict';
    var form = document.getElementById('avatar-crop-form');
    var input = document.getElementById('avatar-file-input');
    var submitBtn = document.getElementById('avatar-submit-btn');
    var modal = document.getElementById('avatar-crop-modal');
    var canvas = document.getElementById('avatar-crop-canvas');
    var zoomSlider = document.getElementById('avatar-crop-zoom');
    var cancelBtn = document.getElementById('avatar-crop-cancel');
    var confirmBtn = document.getElementById('avatar-crop-confirm');
    if (!form || !input || !modal || !canvas) return;
    var ctx = canvas.getContext('2d');
    var img = null;            // HTMLImageElement cargado
    var offsetX = 0, offsetY = 0;  // traslación de la imagen dentro del canvas
    var zoom = 1;              // escala extra (1 = ajustar al círculo)
    var dragging = false;
    var lastX = 0, lastY = 0;
    var CROP = canvas.width;   // 320 — lado del canvas = diámetro del círculo

    // Calcula el tamaño base (zoom=1): la imagen cabe entera dentro del
    // círculo, manteniendo proporción. Así la imagen nunca se sale del
    // recorte por defecto; el usuario acercará si quiere.
    function baseScale() {
        if (!img) return 1;
        return Math.min(CROP / img.width, CROP / img.height);
    }
    function drawScale() { return baseScale() * zoom; }

    function clampOffset() {
        var s = drawScale();
        var w = img.width * s, h = img.height * s;
        // La imagen debe cubrir SIEMPRE el círculo (no puede haber huecos
        // negros a los lados), así que limitamos el offset para que ningún
        // borde entre dentro del área visible [0, CROP].
        var minX = CROP - w, maxX = 0;
        var minY = CROP - h, maxY = 0;
        if (minX > maxX) { offsetX = (minX + maxX) / 2; }  // img más estrecha que crop: centrar
        else { offsetX = Math.max(minX, Math.min(maxX, offsetX)); }
        if (minY > maxY) { offsetY = (minY + maxY) / 2; }
        else { offsetY = Math.max(minY, Math.min(maxY, offsetY)); }
    }

    function redraw() {
        if (!img) return;
        clampOffset();
        ctx.clearRect(0, 0, CROP, CROP);
        ctx.drawImage(img, offsetX, offsetY, img.width * drawScale(), img.height * drawScale());
    }

    function loadImage(file) {
        if (!file.type.startsWith('image/')) { return; }
        if (file.size > 5 * 1024 * 1024) {
            alert('La imagen supera los 5MB.');
            return;
        }
        var reader = new FileReader();
        reader.onload = function(e) {
            var newImg = new Image();
            newImg.onload = function() {
                img = newImg;
                offsetX = 0; offsetY = 0; zoom = 1;
                zoomSlider.value = '100';
                redraw();
                openModal();
            };
            newImg.src = e.target.result;
        };
        reader.readAsDataURL(file);
    }

    function openModal() {
        modal.hidden = false;
        void modal.offsetWidth;  // reflow para la transición
        modal.classList.add('modal-visible');
        document.body.style.overflow = 'hidden';
    }
    function closeModal() {
        modal.classList.remove('modal-visible');
        document.body.style.overflow = '';
        setTimeout(function() { modal.hidden = true; }, 200);
    }

    input.addEventListener('change', function() {
        if (input.files && input.files[0]) loadImage(input.files[0]);
    });

    // Drag con ratón y touch.
    function startDrag(x, y) { dragging = true; lastX = x; lastY = y; }
    function moveDrag(x, y) {
        if (!dragging || !img) return;
        offsetX += (x - lastX); offsetY += (y - lastY);
        lastX = x; lastY = y;
        redraw();
    }
    function endDrag() { dragging = false; }
    canvas.addEventListener('mousedown', function(e) { startDrag(e.clientX, e.clientY); });
    window.addEventListener('mousemove', function(e) { moveDrag(e.clientX, e.clientY); });
    window.addEventListener('mouseup', endDrag);
    canvas.addEventListener('touchstart', function(e) {
        if (e.touches[0]) { startDrag(e.touches[0].clientX, e.touches[0].clientY); }
    }, { passive: true });
    canvas.addEventListener('touchmove', function(e) {
        if (e.touches[0]) { moveDrag(e.touches[0].clientX, e.touches[0].clientY); e.preventDefault(); }
    }, { passive: false });
    canvas.addEventListener('touchend', endDrag);

    zoomSlider.addEventListener('input', function() {
        zoom = parseInt(zoomSlider.value, 10) / 100;
        redraw();
    });

    cancelBtn.addEventListener('click', function() {
        closeModal();
        input.value = '';  // resetea el input para poder re-elegir la misma
    });

    confirmBtn.addEventListener('click', function() {
        if (!img) return;
        closeModal();
        // Dispara el submit del form: el handler de submit (más abajo) lee
        // el canvas, genera el Blob PNG y lo POSTea con fetch.
        form.requestSubmit ? form.requestSubmit() : form.submit();
    });

    // Intercepta el submit: si hay un recorte listo, construye un FormData
    // con el Blob PNG y lo POSTea con fetch (siguiendo el redirect del
    // backend a /dashboard?ok=avatar). Así el backend recibe un multipart
    // "avatar" real, idéntico al del input type=file original.
    form.addEventListener('submit', function(e) {
        if (!img) {
            // Sin imagen cargada (p.ej. JS deshabilitado): deja que el form
            // se envíe normal con el input type=file.
            return;
        }
        e.preventDefault();
        // Dibuja el recorte cuadrado (lo que se ve en el canvas = el círculo)
        // en un canvas oculto a 400px (igual que el backend redimensiona).
        var outSize = 400;
        var out = document.createElement('canvas');
        out.width = outSize; out.height = outSize;
        var octx = out.getContext('2d');
        octx.drawImage(canvas, 0, 0, CROP, CROP, 0, 0, outSize, outSize);
        out.toBlob(function(blob) {
            if (!blob) { alert('No se pudo generar el recorte.'); return; }
            var fd = new FormData();
            fd.append('avatar', blob, 'avatar.png');
            fetch(form.action, { method: 'POST', body: fd, redirect: 'follow' })
                .then(function() { window.location.href = '/dashboard?ok=avatar'; })
                .catch(function() { alert('Error al subir la foto.'); });
        }, 'image/png');
    });
})();
