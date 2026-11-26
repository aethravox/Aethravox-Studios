import { S } from './state.js';

    // ====== Helpers de formato para archivos / voz ======
    export function formatBytes(n) {
        if (!n || n <= 0) return '0 B';
        const units = ['B', 'KB', 'MB', 'GB'];
        let i = 0, v = n;
        while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
        return (v < 10 ? v.toFixed(1) : Math.round(v)) + ' ' + units[i];
    }
    export function formatDuration(secs) {
        const m = Math.floor(secs / 60);
        const s = secs % 60;
        return m + ':' + (s < 10 ? '0' + s : s);
    }
    export function fileIcon(mime, name) {
        if (mime.startsWith('video/')) return '🎬';
        if (mime.startsWith('audio/')) return '🎵';
        if (mime.startsWith('image/')) return '🖼️';
        if (mime === 'application/pdf') return '📕';
        if (mime.includes('zip') || mime.includes('compressed')) return '🗜️';
        if (mime.startsWith('text/') || mime === 'application/json') return '📄';
        if (mime.includes('word') || name.endsWith('.doc') || name.endsWith('.docx')) return '📝';
        if (mime.includes('sheet') || name.endsWith('.xls') || name.endsWith('.xlsx')) return '📊';
        if (mime.includes('presentation') || name.endsWith('.ppt') || name.endsWith('.pptx')) return '📑';
        return '📎';
    }

