import { S } from './state.js';
import { fileIcon, formatBytes, formatDuration } from './Helpers_de_formato.js';
import { escapeHtml } from './Utilidades.js';

    // ====== Body del mensaje: texto, imagen, archivo, nota de voz o "eliminado" ======
    export function formatBody(body, deleted, edited, isSelf) {
        if (deleted) {
            return '<span class="chat-msg-deleted">Mensaje eliminado</span>';
        }
        let inner;
        if (body.startsWith('[IMG]')) {
            const src = body.substring(5);
            inner = `<img src="${escapeHtml(src)}" alt="Imagen" loading="lazy"
                     style="max-width:200px;max-height:200px;border-radius:8px;cursor:pointer;display:block;object-fit:cover;"
                     onclick="window.openImageFullscreen('${src}')">`;
        } else if (body.startsWith('[VOICE]')) {
            // [VOICE]/chat/files/<file>:<duration_secs>
            const rest = body.substring(7);
            const colonIdx = rest.lastIndexOf(':');
            const src = colonIdx >= 0 ? rest.substring(0, colonIdx) : rest;
            const durSecs = colonIdx >= 0 ? parseInt(rest.substring(colonIdx + 1), 10) || 0 : 0;
            const durStr = durSecs > 0 ? formatDuration(durSecs) : '';
            inner = `<div class="chat-voice-note">
                <span class="chat-voice-mic">🎙️</span>
                <audio controls preload="metadata" src="${escapeHtml(src)}"
                       style="height:32px;max-width:220px;"></audio>
                ${durStr ? `<span class="chat-voice-dur">${durStr}</span>` : ''}
            </div>`;
        } else if (body.startsWith('[FILE]')) {
            // [FILE]/chat/files/<file>:<orig_name>:<mime>:<size>
            const rest = body.substring(6);
            const parts = rest.split(':');
            // orig_name puede contener ":" teóricamente — lo reconstruimos a partir de parts[1] hasta len-2
            const src = parts[0] || '';
            const size = parts.length >= 4 ? parseInt(parts[parts.length - 1], 10) || 0 : 0;
            const mime = parts.length >= 3 ? parts[parts.length - 2] : '';
            const origName = parts.length >= 3
                ? parts.slice(1, parts.length - 2).join(':') || 'archivo'
                : (parts[1] || 'archivo');
            const sizeStr = formatBytes(size);
            const icon = fileIcon(mime, origName);
            inner = `<a class="chat-file-card" href="${escapeHtml(src)}" target="_blank" rel="noopener" download>
                <span class="chat-file-icon">${icon}</span>
                <span class="chat-file-meta">
                    <span class="chat-file-name">${escapeHtml(origName)}</span>
                    <span class="chat-file-size">${escapeHtml(mime || 'archivo')} · ${escapeHtml(sizeStr)}</span>
                </span>
                <span class="chat-file-dl">⬇</span>
            </a>`;
        } else {
            inner = escapeHtml(body);
        }
        const editedBadge = edited ? '<span class="chat-msg-edited-badge" title="Mensaje editado">editado</span>' : '';
        return inner + editedBadge;
    }

