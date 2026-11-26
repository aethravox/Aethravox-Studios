import { S } from './state.js';
import { showToast } from './Utilidades.js';

    // ====== Modal fullscreen para ver imagen ======
    // Visor con: flecha para salir (←), zoom con rueda del mouse anclado al
    // cursor, arrastrar para panear, doble-click para toggle 1x↔2x, y menú ⋯
    // con Guardar / Copiar enlace / Abrir en pestaña nueva / Compartir /
    // Rotar / Reiniciar zoom. Cierra con Esc o click fuera de la imagen.
    window.openImageFullscreen = function(src) {
        // Si ya hay uno abierto, lo cerramos primero (no apilamos).
        document.querySelectorAll('.image-fullscreen-overlay').forEach(o => o.remove());

        const overlay = document.createElement('div');
        overlay.className = 'image-fullscreen-overlay';
        // Extraemos un nombre de archivo legible a partir del src.
        const fname = (() => {
            try {
                const u = new URL(src, window.location.origin);
                const last = u.pathname.split('/').filter(Boolean).pop();
                return last ? decodeURIComponent(last) : 'Imagen';
            } catch { return 'Imagen'; }
        })();

        overlay.innerHTML = `
            <div class="ivf-toolbar">
                <button type="button" class="ivf-btn ivf-close" title="Volver (Esc)" aria-label="Volver">←</button>
                <div class="ivf-title" title="${fname}">${fname}</div>
                <button type="button" class="ivf-btn ivf-menu" title="Más opciones" aria-label="Más opciones">⋯</button>
                <div class="ivf-menu-dropdown" role="menu" hidden>
                    <button type="button" role="menuitem" data-act="save">⬇️ Guardar imagen</button>
                    <button type="button" role="menuitem" data-act="copy">🔗 Copiar enlace</button>
                    <button type="button" role="menuitem" data-act="open">↗️ Abrir en pestaña nueva</button>
                    <button type="button" role="menuitem" data-act="share">📤 Compartir</button>
                    <button type="button" role="menuitem" data-act="rotate">⟳ Rotar 90°</button>
                    <button type="button" role="menuitem" data-act="reset">🔍 Reiniciar zoom</button>
                </div>
            </div>
            <div class="ivf-stage">
                <img class="image-fullscreen-img" src="${src}" alt="${fname}" draggable="false">
            </div>
            <div class="ivf-zoombar">
                <button type="button" class="ivf-btn" data-zoom="out" title="Alejar">−</button>
                <span class="ivf-zoom-pct">100%</span>
                <button type="button" class="ivf-btn" data-zoom="in" title="Acercar">+</button>
                <button type="button" class="ivf-btn" data-zoom="fit" title="Ajustar / reiniciar">⟳</button>
            </div>
        `;
        document.body.appendChild(overlay);
        document.body.style.overflow = 'hidden';

        const img      = overlay.querySelector('.image-fullscreen-img');
        const stage    = overlay.querySelector('.ivf-stage');
        const pctEl    = overlay.querySelector('.ivf-zoom-pct');
        const menuBtn  = overlay.querySelector('.ivf-menu');
        const menu     = overlay.querySelector('.ivf-menu-dropdown');

        // Estado de la transformación.
        let scale = 1, rotation = 0, tx = 0, ty = 0;
        let dragging = false, sx = 0, sy = 0, startTx = 0, startTy = 0;

        function applyTransform() {
            img.style.transform =
                `translate(${tx}px, ${ty}px) rotate(${rotation}deg) scale(${scale})`;
            pctEl.textContent = Math.round(scale * 100) + '%';
            stage.style.cursor = scale > 1
                ? (dragging ? 'grabbing' : 'grab')
                : 'zoom-in';
        }

        // Zoom anclado al cursor: el punto bajo el cursor permanece fijo.
        // delta = (cursor - stageCenter); t' = delta*(1 - s'/s) + t.
        function setZoom(newScale, cx, cy) {
            newScale = Math.max(0.5, Math.min(8, newScale));
            if (cx !== undefined && cy !== undefined && newScale !== scale) {
                const r = stage.getBoundingClientRect();
                const dx = cx - (r.left + r.width / 2);
                const dy = cy - (r.top + r.height / 2);
                const k = newScale / scale;
                tx = dx * (1 - k) + tx;
                ty = dy * (1 - k) + ty;
            }
            scale = newScale;
            applyTransform();
        }

        function resetView() {
            scale = 1; rotation = 0; tx = 0; ty = 0;
            applyTransform();
        }

        // ====== Cerrar ======
        function close() {
            overlay.remove();
            document.body.style.overflow = '';
            document.removeEventListener('keydown', onKey);
        }
        function onKey(e) {
            if (e.key === 'Escape') { close(); }
            else if (e.key === '+' || e.key === '=') { setZoom(scale * 1.25); }
            else if (e.key === '-') { setZoom(scale / 1.25); }
            else if (e.key === '0') { resetView(); }
            else if (e.key === 'r' || e.key === 'R') {
                rotation = (rotation + 90) % 360; applyTransform();
            }
        }
        document.addEventListener('keydown', onKey);

        // Click en fondo (no en imagen/toolbar/zoombar) → cerrar.
        overlay.addEventListener('click', (e) => {
            if (e.target === overlay || e.target === stage) close();
        });
        overlay.querySelector('.ivf-close').addEventListener('click', close);

        // ====== Menú ⋯ ======
        menuBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            const open = !menu.hidden;
            menu.hidden = open;
        });
        menu.addEventListener('click', (e) => e.stopPropagation());
        document.addEventListener('click', () => { if (!menu.hidden) menu.hidden = true; });

        // ====== Acciones del menú ======
        async function downloadImage() {
            try {
                // Fetch como blob para forzar descarga (same-origin / same-site).
                const resp = await fetch(src, { credentials: 'same-origin' });
                if (!resp.ok) throw new Error('HTTP ' + resp.status);
                const blob = await resp.blob();
                const url = URL.createObjectURL(blob);
                const a = document.createElement('a');
                a.href = url;
                a.download = fname || 'imagen';
                document.body.appendChild(a);
                a.click();
                a.remove();
                setTimeout(() => URL.revokeObjectURL(url), 1000);
                showToast('Imagen guardada', 'success', 2000);
            } catch (err) {
                // Fallback: abrir en pestaña nueva (el usuario la guarda a mano).
                window.open(src, '_blank');
                showToast('No se pudo descargar directamente: abierta en pestaña nueva', 'info', 3000);
            }
        }
        async function copyLink() {
            const full = new URL(src, window.location.origin).href;
            try {
                await navigator.clipboard.writeText(full);
                showToast('Enlace copiado', 'success', 2000);
            } catch {
                // Fallback para navegadores sin Clipboard API.
                const ta = document.createElement('textarea');
                ta.value = full; document.body.appendChild(ta);
                ta.select();
                try { document.execCommand('copy'); showToast('Enlace copiado', 'success', 2000); }
                catch { showToast('No se pudo copiar el enlace', 'error', 2500); }
                ta.remove();
            }
        }
        function openInNewTab() { window.open(src, '_blank'); }
        async function shareImage() {
            const full = new URL(src, window.location.origin).href;
            if (navigator.share) {
                try {
                    // Si el navegador soporta compartir archivos, mandamos el blob.
                    try {
                        const resp = await fetch(src, { credentials: 'same-origin' });
                        if (resp.ok) {
                            const blob = await resp.blob();
                            const file = new File([blob], fname, { type: blob.type });
                            if (navigator.canShare && navigator.canShare({ files: [file] })) {
                                await navigator.share({ files: [file], title: fname });
                                return;
                            }
                        }
                    } catch {}
                    await navigator.share({ title: fname, url: full });
                } catch (err) {
                    if (err && err.name !== 'AbortError') {
                        showToast('No se pudo compartir', 'error', 2500);
                    }
                }
            } else {
                copyLink();
            }
        }

        menu.querySelectorAll('[data-act]').forEach(btn => {
            btn.addEventListener('click', async () => {
                menu.hidden = true;
                const act = btn.dataset.act;
                if (act === 'save')   await downloadImage();
                else if (act === 'copy')   copyLink();
                else if (act === 'open')   openInNewTab();
                else if (act === 'share')  await shareImage();
                else if (act === 'rotate') { rotation = (rotation + 90) % 360; applyTransform(); }
                else if (act === 'reset')  resetView();
            });
        });

        // ====== Zoom con rueda del mouse (anclado al cursor) ======
        stage.addEventListener('wheel', (e) => {
            e.preventDefault();
            const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
            setZoom(scale * factor, e.clientX, e.clientY);
        }, { passive: false });

        // ====== Doble click → toggle 1x ↔ 2x ======
        img.addEventListener('dblclick', (e) => {
            setZoom(scale > 1.5 ? 1 : 2, e.clientX, e.clientY);
        });

        // ====== Arrastrar para panear (solo si hay zoom) ======
        img.addEventListener('mousedown', (e) => {
            if (scale <= 1) return;
            e.preventDefault();
            dragging = true;
            sx = e.clientX; sy = e.clientY;
            startTx = tx; startTy = ty;
            applyTransform();
        });
        window.addEventListener('mousemove', (e) => {
            if (!dragging) return;
            tx = startTx + (e.clientX - sx);
            ty = startTy + (e.clientY - sy);
            applyTransform();
        });
        window.addEventListener('mouseup', () => {
            if (!dragging) return;
            dragging = false;
            applyTransform();
        });

        // Touch: pan con un dedo cuando hay zoom; pinch-zoom con dos.
        let pinchStartDist = 0, pinchStartScale = 1;
        img.addEventListener('touchstart', (e) => {
            if (e.touches.length === 2) {
                const dx = e.touches[0].clientX - e.touches[1].clientX;
                const dy = e.touches[0].clientY - e.touches[1].clientY;
                pinchStartDist = Math.hypot(dx, dy);
                pinchStartScale = scale;
            } else if (e.touches.length === 1 && scale > 1) {
                dragging = true;
                sx = e.touches[0].clientX; sy = e.touches[0].clientY;
                startTx = tx; startTy = ty;
            }
        }, { passive: true });
        img.addEventListener('touchmove', (e) => {
            if (e.touches.length === 2 && pinchStartDist > 0) {
                e.preventDefault();
                const dx = e.touches[0].clientX - e.touches[1].clientX;
                const dy = e.touches[0].clientY - e.touches[1].clientY;
                const d = Math.hypot(dx, dy);
                setZoom(pinchStartScale * (d / pinchStartDist));
            } else if (e.touches.length === 1 && dragging) {
                e.preventDefault();
                tx = startTx + (e.touches[0].clientX - sx);
                ty = startTy + (e.touches[0].clientY - sy);
                applyTransform();
            }
        }, { passive: false });
        img.addEventListener('touchend', () => {
            dragging = false;
            pinchStartDist = 0;
        });

        // ====== Botones de la zoombar ======
        overlay.querySelector('[data-zoom="out"]').addEventListener('click', () => setZoom(scale / 1.25));
        overlay.querySelector('[data-zoom="in"]').addEventListener('click', () => setZoom(scale * 1.25));
        overlay.querySelector('[data-zoom="fit"]').addEventListener('click', resetView);

        applyTransform();
    };

