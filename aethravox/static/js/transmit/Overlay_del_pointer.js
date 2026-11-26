import { S } from './state.js';
import { bitrateForQuality } from './Bitrate.js';

    // ====== Overlay del pointer del receptor con control ======

    export function hidePointerOverlay() {
        const ov = document.getElementById('pointer-overlay');
        ov.hidden = true;
    }

    export function showPointer(x, y, display) {
        const ov = document.getElementById('pointer-overlay');
        ov.hidden = false;
        // x, y están en normalized (0-1) → convertir a % del preview-wrap.
        ov.style.left = (x * 100) + '%';
        ov.style.top = (y * 100) + '%';
        if (display) {
            document.getElementById('pointer-name').textContent = display;
        }
        // Auto-ocultar tras 2s sin moverse (evita pointer colgado).
        clearTimeout(pointerHideTimer);
        pointerHideTimer = setTimeout(hidePointerOverlay, 2000);
    }

    export function showClickRipple(x, y) {
        const container = document.getElementById('ripple-container');
        const ripple = document.createElement('div');
        ripple.className = 'transmit-click-ripple';
        ripple.style.left = (x * 100) + '%';
        ripple.style.top = (y * 100) + '%';
        container.appendChild(ripple);
        setTimeout(() => ripple.remove(), 800);
    }

    export function handleControlInput(d) {
        // d.input_type: 'move' | 'click' | 'scroll'
        // d.x, d.y: coords normalized 0-1
        // d.from: peer_id del controller
        const x = parseFloat(d.x || 0);
        const y = parseFloat(d.y || 0);
        const display = peerDisplayMap.get(d.from) || 'Receptor';
        if (d.input_type === 'move') {
            showPointer(x, y, display);
        } else if (d.input_type === 'click') {
            showClickRipple(x, y);
            showPointer(x, y, display);
            // Intento de inyección real (solo funciona si el Aethravox page
            // es lo que se está compartiendo — caso raro).
            tryInjectClick(x, y);
        }
    }

    // Intento de click injection via document.elementFromPoint.
    // Funciona SOLO si la pestaña compartida es la de Aethravox misma.
    // En cualquier otro caso (pantalla completa, otra pestaña, app nativa)
    // el click se ignora silenciosamente — el ripple sigue siendo visible.
    export function tryInjectClick(nx, ny) {
        const video = document.getElementById('sender-preview');
        if (!video || video.hidden) return;
        const rect = video.getBoundingClientRect();
        // Mapear coords normalizadas (0-1) → coords reales en la ventana.
        // El video está en este documento → podemos usar elementFromPoint.
        const realX = rect.left + nx * rect.width;
        const realY = rect.top + ny * rect.height;
        try {
            const el = document.elementFromPoint(realX, realY);
            if (el && el !== video) {
                // Dispatch synthetic click
                el.click();
            }
        } catch (e) {
            // Silencioso — solo un intento de bonus.
        }
    }

    // ====== Crear PeerConnection para un receptor nuevo ======
    export async function createPeerForReceiver(peerId, peerDisplay) {
        const pc = new RTCPeerConnection({ iceServers: ICE_SERVERS });
        peerConnections.set(peerId, pc);

        // Añadir tracks locales (screen sharing)
        if (localStream) {
            localStream.getTracks().forEach(track => {
                const sender = pc.addTrack(track, localStream);
                // Aplicar bitrate cap según calidad elegida
                const maxBitrate = bitrateForQuality(chosenQuality);
                if (maxBitrate > 0) {
                    const params = sender.getParameters();
                    if (!params.encodings) params.encodings = [{}];
                    params.encodings[0].maxBitrate = maxBitrate;
                    sender.setParameters(params).catch(e => console.warn('setParameters:', e));
                }
            });
        }

        // Enviar ICE candidates al receptor
        pc.onicecandidate = (e) => {
            if (e.candidate && ws && ws.readyState === 1) {
                ws.send(JSON.stringify({
                    type: 'ice_candidate',
                    to: peerId,
                    candidate: e.candidate.toJSON(),
                }));
            }
        };

        // Telemetría (opcional, para debugging)
        pc.onconnectionstatechange = () => {
            console.log(`[peer ${peerId}] ${pc.connectionState}`);
            const item = document.querySelector('.transmit-receiver-item[data-peer-id="' + peerId + '"]');
            if (item) {
                const statusEl = item.querySelector('.transmit-receiver-status');
                if (statusEl) {
                    statusEl.textContent = pc.connectionState;
                    statusEl.style.color = pc.connectionState === 'connected' ? '#10b981' :
                        pc.connectionState === 'failed' ? '#ef4444' : '#888';
                }
            }
        };

        // Crear offer
        try {
            const offer = await pc.createOffer({ offerToReceiveVideo: false, offerToReceiveAudio: false });
            await pc.setLocalDescription(offer);
            ws.send(JSON.stringify({
                type: 'offer',
                to: peerId,
                sdp: offer.sdp,
            }));
        } catch (e) {
            console.error('createOffer:', e);
        }
    }

