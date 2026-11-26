import { S } from './state.js';
import { createPeerForReceiver, handleControlInput, hidePointerOverlay } from './Overlay_del_pointer.js';
import { stopSharing } from './Detener_transmisi_n.js';
import { escapeHtml, showToast } from './Utilidades.js';
import { addControlRequest, renderControlPanel, updateControllerDisplay } from './Gesti_n_del_panel_de_control.js';

    // ====== Recibir WS ======
    export async function handleWs(d) {
        if (d.type === 'room_created') {
            S.currentRoomName = d.name;
            showToast('Sala creada. Comparte el nombre + contraseña con quien quieras que vea.', 'success', 5000);
        } else if (d.type === 'peer_joined') {
            // Un receptor se conectó → crear PeerConnection y enviarle offer.
            S.peerDisplayMap.set(d.peer_id, d.peer_display);
            await createPeerForReceiver(d.peer_id, d.peer_display);
            document.getElementById('receiver-count').textContent =
                String(S.peerConnections.size);
            // Añadir a la lista visual
            const list = document.getElementById('receivers-list');
            if (list.querySelector('.muted')) list.innerHTML = '';
            const item = document.createElement('div');
            item.className = 'transmit-receiver-item';
            item.dataset.peerId = d.peer_id;
            item.innerHTML = '<span class="transmit-receiver-avatar">👤</span>' +
                '<span class="transmit-receiver-name">' + escapeHtml(d.peer_display || 'Receptor') + '</span>' +
                '<span class="transmit-receiver-status">conectado</span>';
            list.appendChild(item);
        } else if (d.type === 'peer_left') {
            // El receptor se fue → cerrar su PeerConnection.
            const pc = S.peerConnections.get(d.peer_id);
            if (pc) {
                pc.close();
                S.peerConnections.delete(d.peer_id);
            }
            // Limpiar su display + solicitud pendiente + si era controller.
            S.peerDisplayMap.delete(d.peer_id);
            S.pendingRequests.delete(d.peer_id);
            if (S.currentControllerId === d.peer_id) {
                S.currentControllerId = null;
                hidePointerOverlay();
            }
            renderControlPanel();
            document.getElementById('receiver-count').textContent =
                String(S.peerConnections.size);
            const item = document.querySelector('.transmit-receiver-item[data-peer-id="' + d.peer_id + '"]');
            if (item) {
                item.querySelector('.transmit-receiver-status').textContent = 'desconectado';
                item.style.opacity = '0.5';
                setTimeout(() => item.remove(), 2000);
            }
        } else if (d.type === 'answer') {
            // El receptor respondió al offer → setRemoteDescription
            const pc = S.peerConnections.get(d.from);
            if (pc && pc.signalingState !== 'stable') {
                try { await pc.setRemoteDescription(new RTCSessionDescription({ type: 'answer', sdp: d.sdp })); }
                catch (e) { console.error('setRemote answer:', e); }
            }
        } else if (d.type === 'ice_candidate') {
            const pc = S.peerConnections.get(d.from);
            if (pc) {
                try { await pc.addIceCandidate(new RTCIceCandidate(d.candidate)); }
                catch (e) { console.error('addIce:', e); }
            }
        } else if (d.type === 'error') {
            showToast(d.message || 'Error', 'error');
            // Si el error es al crear la sala, volver al form
            if (S.localStream) {
                stopSharing();
            }
        } else if (d.type === 'control_requested') {
            // Un receptor pidió control → añadir a la cola visual.
            addControlRequest(d.peer_id, d.peer_display);
        } else if (d.type === 'controller_changed') {
            // Cambió quién tiene control (puede ser null = nadie).
            updateControllerDisplay(d.controller_id, d.controller_display);
        } else if (d.type === 'control_input') {
            // Input del receptor que tiene control → renderizar pointer/ripple.
            handleControlInput(d);
        }
    }

