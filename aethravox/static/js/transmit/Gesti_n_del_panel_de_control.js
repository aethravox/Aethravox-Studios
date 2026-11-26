import { S } from './state.js';
import { showToast } from './Utilidades.js';

    // ====== Gestión del panel de control remoto ======
    // Map de peer_id → display (para nombres en la UI).
    // Estado local: quién tiene control (null = nadie).
    // Estado local: set de peer_ids con solicitud pendiente.

    export function addControlRequest(peerId, display) {
        S.pendingRequests.add(peerId);
        S.peerDisplayMap.set(peerId, display);
        renderControlPanel();
        showToast('✋ ' + (display || 'Receptor') + ' pide control', 'info', 4000);
    }

    export function renderControlPanel() {
        const reqContainer = document.getElementById('control-requests');
        if (S.pendingRequests.size === 0) {
            reqContainer.innerHTML = '';
        } else {
            reqContainer.innerHTML = Array.from(S.pendingRequests).map(pid => {
                const name = S.peerDisplayMap.get(pid) || 'Receptor';
                return `<div class="transmit-control-request" data-peer-id="${id}">
                    <span class="transmit-control-request-name">✋ ${escapeHtml(name)}</span>
                    <button class="btn btn-primary btn-sm" onclick="approveControl(${pid})">Permitir</button>
                    <button class="btn btn-secondary btn-sm" onclick="denyControl(${pid})">Rechazar</button>
                </div>`;
            }).join('');
        }
        // Estado del controller actual
        const curEl = document.getElementById('control-current');
        if (currentControllerId !== null) {
            const name = peerDisplayMap.get(currentControllerId) || 'Receptor';
            curEl.innerHTML = `<div class="transmit-control-active">
                <span class="transmit-control-active-name">🎮 Controlando: ${escapeHtml(name)}</span>
                <button class="btn btn-danger btn-sm" onclick="revokeControl()">Quitar control</button>
            </div>`;
        } else {
            curEl.innerHTML = '<span class="muted small">Nadie tiene control. Cuando alguien lo pida, verás su solicitud aquí.</span>';
        }
    }

    export function updateControllerDisplay(controllerId, controllerDisplay) {
        currentControllerId = controllerId;
        if (controllerId !== null && controllerId !== undefined) {
            peerDisplayMap.set(controllerId, controllerDisplay);
            pendingRequests.delete(controllerId);
        }
        renderControlPanel();
        // Si ya no hay controller → ocultar overlay.
        if (currentControllerId === null) {
            hidePointerOverlay();
        }
    }

    window.approveControl = function(peerId) {
        if (!ws || ws.readyState !== 1) return;
        ws.send(JSON.stringify({ type: 'control_grant', peer_id: peerId }));
    };
    window.denyControl = function(peerId) {
        if (!ws || ws.readyState !== 1) return;
        ws.send(JSON.stringify({ type: 'control_deny', peer_id: peerId }));
        pendingRequests.delete(peerId);
        renderControlPanel();
    };
    window.revokeControl = function() {
        if (!ws || ws.readyState !== 1) return;
        ws.send(JSON.stringify({ type: 'control_revoke' }));
    };

