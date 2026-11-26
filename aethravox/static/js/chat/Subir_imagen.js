import { S } from './state.js';
import { showToast } from './Utilidades.js';

    // ====== Subir imagen ======
    export async function uploadImage(file) {
        if (!S.currentConv) { showToast('Selecciona una conversación primero', 'warning'); return; }
        if (file.size > 5 * 1024 * 1024) {
            showToast('Imagen demasiado grande (máximo 5MB)', 'error');
            return;
        }
        const formData = new FormData();
        formData.append('image', file);
        try {
            const resp = await fetch('/api/chat/' + S.currentConv + '/upload', {
                method: 'POST',
                body: formData,
            });
            if (!resp.ok) {
                const err = await resp.json().catch(() => ({}));
                showToast(err.error || 'Error al subir imagen', 'error');
                return;
            }
            // El mensaje se añade automáticamente via WS broadcast.
            // No necesitamos añadirlo manualmente aquí.
        } catch (e) {
            showToast('Error de red al subir imagen', 'error');
        }
    }

