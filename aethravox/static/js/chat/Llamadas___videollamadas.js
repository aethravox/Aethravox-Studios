import { S } from './state.js';
import { selectConv } from './Seleccionar_conversaci_n.js';
import { playRingTone, stopRingTone } from './Tono_de_llamada.js';
import { formatDuration } from './Helpers_de_formato.js';
import { showToast } from './Utilidades.js';

    // ====== Llamadas / videollamadas P2P dentro del chat ======
    // Solo DMs. El servidor enruta la señalización; el audio/vídeo viaja P2P.
    // Estado de la llamada local:
    //   - callPC: RTCPeerConnection actual (null si no hay).
    //   - callLocalStream / callRemoteStream: MediaStreams.
    //   - callRole: 'caller' | 'callee'.
    //   - callCurrentId: id de la sesión de señalización (string).
    //   - callTimerInterval: cronómetro.

    export function showCallButtons(show) {
        const el = document.getElementById('chat-call-buttons');
        if (el) el.hidden = !show;
    }

    export function openCallOverlay(statusText) {
        const ov = document.getElementById('call-overlay');
        ov.hidden = false;
        requestAnimationFrame(() => ov.classList.add('call-visible'));
        const s = document.getElementById('call-status');
        if (statusText && s) s.textContent = statusText;
        document.body.style.overflow = 'hidden';
    }
    export function closeCallOverlay() {
        const ov = document.getElementById('call-overlay');
        ov.classList.remove('call-visible');
        setTimeout(() => { ov.hidden = true; }, 200);
        document.body.style.overflow = '';
        // Reset UI
        const remoteVid = document.getElementById('call-remote-video');
        const localVid = document.getElementById('call-local-video');
        if (remoteVid) { remoteVid.srcObject = null; remoteVid.hidden = true; }
        if (localVid) { localVid.srcObject = null; localVid.hidden = true; }
        const camBtn = document.getElementById('call-cam');
        if (camBtn) camBtn.hidden = true;
        const timer = document.getElementById('call-timer');
        if (timer) timer.textContent = '';
        stopCallTimer();
    }
    export function startCallTimer() {
        S.callStartTime = Date.now();
        const el = document.getElementById('call-timer');
        if (!el) return;
        stopCallTimer();
        S.callTimerInterval = setInterval(() => {
            const secs = Math.floor((Date.now() - S.callStartTime) / 1000);
            el.textContent = formatDuration(secs);
        }, 500);
    }
    export function stopCallTimer() {
        if (S.callTimerInterval) { clearInterval(S.callTimerInterval); S.callTimerInterval = null; }
    }
    export function hangupCall(reason) {
        if (!S.callCurrentId) { closeCallOverlay(); return; }
        // Avisar al peer vía WS que colgamos.
        if (S.ws && S.ws.readyState === 1 && S.callPeerConvId) {
            S.ws.send(JSON.stringify({
                type: 'call_end',
                call_id: S.callCurrentId,
                conversation_id: S.callPeerConvId,
            }));
        }
        teardownCall();
        if (reason) showToast(reason, 'info', 2000);
    }
    export function teardownCall() {
        if (S.callPC) { try { S.callPC.close(); } catch(e){} S.callPC = null; }
        if (S.callLocalStream) {
            S.callLocalStream.getTracks().forEach(t => t.stop());
            S.callLocalStream = null;
        }
        S.callRemoteStream = null;
        S.callCurrentId = null;
        S.callPeerConvId = null;
        S.callRole = null;
        S.callMuted = false;
        S.callCamOff = false;
        closeCallOverlay();
    }

    // Crear PeerConnection, añadir tracks locales y configurar handlers.
    export async function createCallPC() {
        S.callPC = new RTCPeerConnection({ iceServers: ICE_SERVERS });
        S.callRemoteStream = new MediaStream();
        const remoteVid = document.getElementById('call-remote-video');
        remoteVid.srcObject = S.callRemoteStream;
        remoteVid.hidden = false;
        if (S.callLocalStream) {
            S.callLocalStream.getTracks().forEach(track => {
                S.callPC.addTrack(track, S.callLocalStream);
            });
        }
        S.callPC.ontrack = (e) => {
            if (S.callRemoteStream) {
                e.streams[0].getTracks().forEach(t => S.callRemoteStream.addTrack(t));
            }
        };
        S.callPC.onicecandidate = (e) => {
            if (e.candidate && S.ws && S.ws.readyState === 1) {
                S.ws.send(JSON.stringify({
                    type: 'call_ice',
                    call_id: S.callCurrentId,
                    conversation_id: S.callPeerConvId,
                    candidate: e.candidate.toJSON(),
                }));
            }
        };
        S.callPC.onconnectionstatechange = () => {
            const s = document.getElementById('call-status');
            if (!s || !S.callPC) return;
            if (S.callPC.connectionState === 'connected') {
                s.textContent = 'Conectado';
                startCallTimer();
            } else if (S.callPC.connectionState === 'disconnected') {
                s.textContent = 'Reconectando…';
            } else if (S.callPC.connectionState === 'failed') {
                s.textContent = 'Falló la conexión';
                hangupCall('La conexión falló');
            }
        };
    }

    // Iniciar llamada: caller. Pide permisos de media según el tipo.
    export async function startCall(callType) {
        if (!S.currentConv) { showToast('Selecciona una conversación primero', 'warning'); return; }
        // Buscar la conv en la lista para confirmar que es DM.
        let conv = S.conversations.find(x => x.id === S.currentConv);
        if (!conv) {
            for (const c of S.communities) {
                const g = (c.groups || []).find(x => x.id === S.currentConv);
                if (g) { conv = { conv_type: 'group', name: g.name }; break; }
            }
        }
        if (!conv) return;
        if (conv.conv_type !== 'dm') {
            showToast('Las llamadas solo están disponibles en mensajes directos', 'info');
            return;
        }
        if (S.callPC) { showToast('Ya tienes una llamada en curso', 'info'); return; }
        S.callIsVideo = (callType === 'video');
        S.callRole = 'caller';
        S.callPeerConvId = S.currentConv;
        // Pedir media
        try {
            S.callLocalStream = await navigator.mediaDevices.getUserMedia({
                audio: true,
                video: S.callIsVideo ? { facingMode: 'user' } : false,
            });
        } catch (err) {
            console.error('getUserMedia call:', err);
            showToast('No se pudo acceder a cámara/micrófono. Revisa los permisos.', 'warning');
            S.callPeerConvId = null;
            S.callRole = null;
            return;
        }
        // Mostrar overlay con estado "Llamando…"
        const nameEl = document.getElementById('call-name');
        if (nameEl) nameEl.textContent = conv.name || 'Contacto';
        const camBtn = document.getElementById('call-cam');
        if (camBtn) camBtn.hidden = !S.callIsVideo;
        // Local thumb
        const localVid = document.getElementById('call-local-video');
        if (S.callIsVideo) {
            localVid.srcObject = S.callLocalStream;
            localVid.hidden = false;
        } else {
            localVid.hidden = true;
        }
        openCallOverlay('Llamando…');
        // Enviar call_start al server. El call_id se recibe con call_ringing.
        if (S.ws && S.ws.readyState === 1) {
            S.ws.send(JSON.stringify({
                type: 'call_start',
                conversation_id: S.callPeerConvId,
                call_type: callType,
            }));
        } else {
            showToast('Sin conexión de chat', 'error');
            teardownCall();
        }
    }
    window.startCall = startCall;

    // Recibimos call_ringing: el server nos dio un call_id. Creamos la PC y
    // esperamos a call_accepted para enviar el SDP offer.
    export async function handleCallRinging(d) {
        S.callCurrentId = d.call_id;
        await createCallPC();
        // El offer lo creamos tras recibir call_accepted (el peer está listo).
    }
    // Recibimos call_accepted: el peer aceptó. Como caller, creamos y enviamos offer.
    export async function handleCallAccepted(d) {
        if (!S.callPC) {
            // Si la PC no se creó (p.ej. no recibimos ringing), la creamos ahora.
            S.callCurrentId = d.call_id;
            await createCallPC();
        }
        try {
            const offer = await S.callPC.createOffer({ offerToReceiveAudio: true, offerToReceiveVideo: S.callIsVideo });
            await S.callPC.setLocalDescription(offer);
            S.ws.send(JSON.stringify({
                type: 'call_sdp',
                call_id: S.callCurrentId,
                conversation_id: S.callPeerConvId,
                sdp_type: 'offer',
                sdp: offer.sdp,
            }));
            const s = document.getElementById('call-status');
            if (s) s.textContent = 'Conectando…';
        } catch (e) {
            console.error('createOffer:', e);
            hangupCall('No se pudo iniciar la llamada');
        }
    }
    // Recibimos call_incoming: alguien nos llama. Mostrar modal.
    export function handleCallIncoming(d) {
        if (S.callPC) {
            // Ya estamos en una llamada: rechazar automáticamente.
            if (S.ws && S.ws.readyState === 1) {
                S.ws.send(JSON.stringify({
                    type: 'call_reject',
                    call_id: d.call_id,
                    conversation_id: d.conversation_id,
                    reason: 'busy',
                }));
            }
            return;
        }
        // Notificación de escritorio persistente: las llamadas son urgentes,
        // así que usamos requireInteraction para que NO se auto-cierre y
        // siempre se vea aunque el usuario esté en otra app. Al clickar, abre
        // la conv (el modal de llamada sigue visible en la pestaña).
        if (window.showDesktopNotification) {
            const callLabel = d.call_type === 'video' ? '📹 Videollamada entrante' : '📞 Llamada entrante';
            window.showDesktopNotification(d.from_name || 'Llamada entrante', callLabel, {
                tag: 'call-' + d.call_id,
                requireInteraction: true,
                always: true,  // una llamada es urgente: notifica siempre
                url: '/chat',
                conversation_id: d.conversation_id,
                type: 'call',
                onclick: function() {
                    // Fallback para notificaciones page-owned (sin SW).
                    if (typeof selectConv === 'function') selectConv(d.conversation_id);
                },
            });
        }
        // Guardar datos de la llamada entrante para cuando el usuario acepte.
        S.pendingIncoming = d;
        const modal = document.getElementById('incoming-call-modal');
        document.getElementById('incoming-call-avatar').src = d.from_avatar || '';
        document.getElementById('incoming-call-name').textContent = d.from_name || 'Contacto';
        document.getElementById('incoming-call-type').textContent =
            d.call_type === 'video' ? '📹 Videollamada' : '📞 Llamada de voz';
        document.getElementById('incoming-call-title').textContent = d.call_type === 'video'
            ? '📹 Videollamada entrante' : '📞 Llamada entrante';
        modal.hidden = false;
        requestAnimationFrame(() => modal.classList.add('modal-visible'));
        // Sonido de llamada: corto beep opcional vía WebAudio (sin assets externos).
        playRingTone();
    }

    export function acceptIncomingCall() {
        if (!S.pendingIncoming) return;
        const d = S.pendingIncoming;
        stopRingTone();
        const modal = document.getElementById('incoming-call-modal');
        modal.classList.remove('modal-visible');
        setTimeout(() => { modal.hidden = true; }, 200);
        S.callCurrentId = d.call_id;
        S.callPeerConvId = d.conversation_id;
        S.callRole = 'callee';
        S.callIsVideo = (d.call_type === 'video');
        // Cargar la conversación en el chat para tener contexto visual.
        if (S.currentConv !== d.conversation_id) {
            selectConv(d.conversation_id);
        }
        // Pedir media
        navigator.mediaDevices.getUserMedia({
            audio: true,
            video: S.callIsVideo ? { facingMode: 'user' } : false,
        }).then(stream => {
            S.callLocalStream = stream;
            const localVid = document.getElementById('call-local-video');
            if (S.callIsVideo) {
                localVid.srcObject = S.callLocalStream;
                localVid.hidden = false;
            } else {
                localVid.hidden = true;
            }
            const camBtn = document.getElementById('call-cam');
            if (camBtn) camBtn.hidden = !S.callIsVideo;
            const nameEl = document.getElementById('call-name');
            if (nameEl) nameEl.textContent = d.from_name || 'Contacto';
            openCallOverlay('Conectando…');
            // Avisar al caller que aceptamos.
            S.ws.send(JSON.stringify({
                type: 'call_accept',
                call_id: S.callCurrentId,
                conversation_id: S.callPeerConvId,
            }));
            // Crear PC para recibir el offer que llegará vía call_sdp.
            return createCallPC();
        }).catch(err => {
            console.error('getUserMedia incoming:', err);
            showToast('No se pudo acceder a cámara/micrófono', 'warning');
            // Avisar al caller que rechazamos por no tener media.
            S.ws.send(JSON.stringify({
                type: 'call_reject',
                call_id: S.callCurrentId,
                conversation_id: S.callPeerConvId,
                reason: 'no-media',
            }));
            teardownCall();
        });
    }
    export function rejectIncomingCall() {
        stopRingTone();
        const modal = document.getElementById('incoming-call-modal');
        modal.classList.remove('modal-visible');
        setTimeout(() => { modal.hidden = true; }, 200);
        if (S.pendingIncoming) {
            S.ws.send(JSON.stringify({
                type: 'call_reject',
                call_id: S.pendingIncoming.call_id,
                conversation_id: S.pendingIncoming.conversation_id,
                reason: 'declined',
            }));
            S.pendingIncoming = null;
        }
    }
    document.getElementById('incoming-accept').addEventListener('click', acceptIncomingCall);
    document.getElementById('incoming-reject').addEventListener('click', rejectIncomingCall);

    // Recibimos call_sdp: offer o answer del peer.
    export async function handleCallSdp(d) {
        if (!S.callPC) {
            // Sin PC: probablemente la creamos tarde. Ignorar.
            return;
        }
        try {
            const desc = new RTCSessionDescription({ type: d.sdp_type, sdp: d.sdp });
            await S.callPC.setRemoteDescription(desc);
            if (d.sdp_type === 'offer' && S.callRole === 'callee') {
                // Como callee, creamos answer.
                const answer = await S.callPC.createAnswer();
                await S.callPC.setLocalDescription(answer);
                S.ws.send(JSON.stringify({
                    type: 'call_sdp',
                    call_id: S.callCurrentId,
                    conversation_id: S.callPeerConvId,
                    sdp_type: 'answer',
                    sdp: answer.sdp,
                }));
                const s = document.getElementById('call-status');
                if (s) s.textContent = 'Conectando…';
            }
        } catch (e) {
            console.error('setRemoteDescription:', e);
        }
    }
    export async function handleCallIce(d) {
        if (!S.callPC) return;
        try {
            if (d.candidate) {
                await S.callPC.addIceCandidate(new RTCIceCandidate(d.candidate));
            }
        } catch (e) {
            console.error('addIceCandidate:', e);
        }
    }
    export function handleCallEnd(d) {
        stopRingTone();
        if (S.callCurrentId === d.call_id) {
            teardownCall();
            showToast('La llamada terminó', 'info', 2000);
        }
    }
    export function handleCallRejected(d) {
        if (S.callCurrentId === d.call_id) {
            teardownCall();
            const reason = d.reason === 'busy'
                ? 'El usuario está en otra llamada'
                : (d.reason === 'no-media' ? 'El usuario no tiene cámara/micrófono disponible' : 'Llamada rechazada');
            showToast(reason, 'info', 3000);
        }
    }
    export function handleCallUnavailable(d) {
        if (S.callCurrentId === d.call_id || (d.conversation_id === S.callPeerConvId && !S.callCurrentId)) {
            teardownCall();
            showToast('El usuario no está conectado', 'info', 3000);
        }
    }

