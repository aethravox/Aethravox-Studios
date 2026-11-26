import { S } from './state.js';

    // ====== Utilidades ======
    export function escapeHtml(s) {
        return String(s)
            .replace(/&/g, '&amp;').replace(/</g, '&lt;')
            .replace(/>/g, '&gt;').replace(/"/g, '&quot;');
    }
    export function showToast(msg, type, dur) {
        type = type || 'info'; dur = dur || 3500;
        const c = document.getElementById('toast-container');
        if (!c) return;
        const t = document.createElement('div');
        t.className = 'toast toast-' + type;
        t.innerHTML = '<span class="toast-icon">' + ({
            success: '✅', info: 'ℹ️', warning: '⚠️', error: '❌'
        }[type] || 'ℹ️') + '</span><span class="toast-text">' + escapeHtml(msg) + '</span>';
        c.appendChild(t);
        requestAnimationFrame(() => t.classList.add('toast-visible'));
        setTimeout(() => { t.classList.remove('toast-visible'); setTimeout(() => t.remove(), 300); }, dur);
    }

