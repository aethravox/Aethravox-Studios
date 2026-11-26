import { S } from './state.js';
import { confirmDeleteMessage, copyMessageText, showMessageInfo, startEditMessage } from './Acciones_del_men_.js';
import { enterSelectMode, updateSelectCount } from './Modo_selecci_n_m_ltiple.js';
import { showToast } from './Utilidades.js';

    // ====== Menú contextual flotante ======
    export function closeMessageMenu() {
        if (S.currentMenu) { S.currentMenu.remove(); S.currentMenu = null; }
        document.removeEventListener('click', closeMessageMenu);
    }
    // ¿Han pasado menos de 15 min desde el envío? Usa el data-created-iso del
    // mensaje (timestamp UTC en ISO 8601). Fallback: si no hay timestamp, permitir.
    export function canEditMessage(el) {
        const iso = el.dataset.createdIso;
        if (!iso) return true;
        const created = new Date(iso);
        if (isNaN(created.getTime())) return true;
        const ageMs = Date.now() - created.getTime();
        return ageMs < 15 * 60 * 1000;  // 15 min en ms
    }
    export function openMessageMenu(el, btn) {
        closeMessageMenu();
        const id = parseInt(el.dataset.id, 10);
        const isSelf = el.dataset.self === '1';
        const deleted = el.dataset.deleted === '1';
        if (deleted) return;

        // Construir opciones según si es propio o ajeno.
        // El backend valida permisos reales (autor + 15min para editar / 30min para
        // borrar propios; can_delete_messages para borrar ajenos).
        const opts = [];
        opts.push({ label: 'Info', icon: 'ℹ', action: () => showMessageInfo(id) });
        opts.push({ label: 'Copiar texto', icon: '⧉', action: () => copyMessageText(el) });
        // "Seleccionar" → entra en modo selección múltiple y marca este mensaje.
        // Es la forma de activar el multi-select (PC: click ⋯; móvil: long-press).
        opts.push({ label: 'Seleccionar', icon: '☑', action: () => {
            enterSelectMode();
            // Pre-seleccionar el mensaje desde el que se abrió el menú.
            S.selectedIds.add(id);
            el.classList.add('is-selected');
            updateSelectCount();
        }});
        // "Editar" solo si es propio Y han pasado menos de 15 min desde el envío.
        // El backend vuelve a validar; aquí solo ocultamos la opción para no confundir.
        const canEdit = isSelf && canEditMessage(el);
        if (isSelf) {
            if (canEdit) {
                opts.push({ label: 'Editar', icon: '✎', action: () => startEditMessage(id, el) });
            } else {
                opts.push({ label: 'Editar (caducó)', icon: '✎', action: () => showToast('Solo se puede editar en los primeros 15 minutos', 'info'), disabled: true });
            }
            opts.push({ label: 'Eliminar', icon: '🗑', action: () => confirmDeleteMessage(id), danger: true });
        } else {
            // Para mensajes ajenos solo ofrecemos "Eliminar" si el usuario tiene permiso
            // (el backend validará de todos modos). Lo dejamos visible para admins de grupo.
            opts.push({ label: 'Eliminar (si tienes permiso)', icon: '🗑', action: () => confirmDeleteMessage(id), danger: true });
        }

        const menu = document.createElement('div');
        menu.className = 'chat-context-menu';
        menu.innerHTML = opts.map((o, i) => `<button data-idx="${i}" class="chat-context-item${o.danger ? ' danger' : ''}${o.disabled ? ' is-disabled' : ''}"${o.disabled ? ' disabled' : ''}>
            <span class="chat-context-icon">${o.icon}</span> <span>${escapeHtml(o.label)}</span></button>`).join('');
        document.body.appendChild(menu);

        // Posicionar cerca del botón ⋯
        const rect = btn.getBoundingClientRect();
        let left = rect.left - menu.offsetWidth + rect.width;
        let top = rect.top + rect.height + 4;
        // Si se sale por la derecha, moverlo
        if (left < 4) left = rect.right + 4;
        // Si se sale por abajo, mostrar arriba
        if (top + menu.offsetHeight > window.innerHeight - 4) top = rect.top - menu.offsetHeight - 4;
        menu.style.left = left + 'px';
        menu.style.top = top + 'px';

        // Bindear clicks
        menu.querySelectorAll('.chat-context-item').forEach((b, i) => {
            b.addEventListener('click', (ev) => {
                ev.stopPropagation();
                if (b.disabled || opts[i].disabled) { return; }
                opts[i].action();
                closeMessageMenu();
            });
        });

        S.currentMenu = menu;
        // Cerrar al hacer click fuera
        setTimeout(() => document.addEventListener('click', closeMessageMenu), 0);
    }
    window.closeMessageMenu = closeMessageMenu;

