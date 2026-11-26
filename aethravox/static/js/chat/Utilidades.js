import { S } from './state.js';

    // ====== Utilidades ======
    export function escapeHtml(s) {
        return String(s)
            .replace(/&/g, '&amp;').replace(/</g, '&lt;')
            .replace(/>/g, '&gt;').replace(/"/g, '&quot;');
    }
    export function showToast(message, type, duration) {
        type = type || 'info';
        duration = duration || 3500;
        const container = document.getElementById('toast-container');
        if (!container) return;
        const toast = document.createElement('div');
        toast.className = 'toast toast-' + type;
        toast.innerHTML = '<span class="toast-icon">' + ({
            success: '✅', info: 'ℹ️', warning: '⚠️', error: '❌'
        }[type] || 'ℹ️') + '</span><span class="toast-text">' + escapeHtml(message) + '</span>';
        container.appendChild(toast);
        requestAnimationFrame(() => toast.classList.add('toast-visible'));
        setTimeout(() => {
            toast.classList.remove('toast-visible');
            setTimeout(() => toast.remove(), 300);
        }, duration);
    }

