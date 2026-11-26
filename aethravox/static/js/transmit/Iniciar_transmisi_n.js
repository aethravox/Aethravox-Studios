import { S } from './state.js';
import { dimensionsForQuality } from './Bitrate.js';
import { connectWS } from './Conectar_WS.js';
import { stopSharing } from './Detener_transmisi_n.js';
import { updateStatus } from './Status_pill.js';
import { showToast } from './Utilidades.js';

    // ====== Iniciar transmisión ======
    export async function startSharing() {
        const nameInput = document.getElementById('room-name');
        const passInput = document.getElementById('room-password');
        const qSelect = document.getElementById('quality-select');
        const fpsSelect = document.getElementById('fps-select');

        const name = nameInput.value.trim();
        const password = passInput.value.trim();
        S.chosenQuality = qSelect.value;
        S.chosenFps = fpsSelect.value;

        if (!name || !password) {
            showToast('Pon nombre y contraseña', 'warning');
            return;
        }

        // 1. Elegir fuente: pantalla (getDisplayMedia) o cámara (getUserMedia).
        // getDisplayMedia NO existe en Android Chrome / iOS Safari — solo escritorio.
        // En móvil forzamos cámara como fallback para que Transmitir funcione.
        const sourceSelect = document.getElementById('source-select');
        const dims = dimensionsForQuality(S.chosenQuality);
        const fpsInt = parseInt(S.chosenFps, 10);
        const wantsScreen = sourceSelect.value === 'screen';
        const canScreen = !!(navigator.mediaDevices && navigator.mediaDevices.getDisplayMedia);
        const isMobile = /Android|iPhone|iPad|iPod/i.test(navigator.userAgent);

        let useScreen = wantsScreen && canScreen;
        // Si el usuario pidió pantalla pero el navegador no lo soporta (móvil), avisamos
        // y forzamos cámara — así la página sigue siendo útil en Android.
        if (wantsScreen && !canScreen) {
            showToast('Tu navegador no permite compartir pantalla. Usando la cámara.', 'warning');
            useScreen = false;
            sourceSelect.value = 'camera';
        }

        let constraints;
        let stream;
        try {
            if (useScreen) {
                constraints = {
                    video: dims ? {
                        width: { ideal: dims[0] },
                        height: { ideal: dims[1] },
                        frameRate: { ideal: fpsInt },
                    } : { frameRate: { ideal: fpsInt } },
                    audio: false,  // screen audio puede causar eco, lo dejamos fuera por ahora
                };
                stream = await navigator.mediaDevices.getDisplayMedia(constraints);
            } else {
                // Cámara del dispositivo (móvil o escritorio con webcam). En móvil
                // usamos facingMode:'environment' (cámara trasera) por defecto,
                // que suele ser lo que quiere alguien que transmite desde el móvil.
                constraints = {
                    video: {
                        width: { ideal: dims ? dims[0] : 640 },
                        height: { ideal: dims ? dims[1] : 480 },
                        frameRate: { ideal: fpsInt },
                        facingMode: isMobile ? { ideal: 'environment' } : 'user',
                    },
                    audio: false,
                };
                stream = await navigator.mediaDevices.getUserMedia(constraints);
            }
            S.localStream = stream;
        } catch (err) {
            console.error('captura de media:', err);
            showToast(useScreen
                ? 'No se pudo acceder a la pantalla. ¿Cancelaste el permiso?'
                : 'No se pudo acceder a la cámara. Revisa los permisos del navegador.', 'warning');
            return;
        }

        // Si el usuario detiene el share via el botón nativo del browser:
        S.localStream.getVideoTracks()[0].addEventListener('ended', () => {
            console.log('Captura detenida por el browser');
            stopSharing();
        });

        // 2. Cambiar UI
        document.getElementById('sender-form-card').hidden = true;
        document.getElementById('sender-live-card').hidden = false;
        document.getElementById('preview-empty').hidden = true;
        const preview = document.getElementById('sender-preview');
        preview.srcObject = S.localStream;
        preview.hidden = false;
        document.getElementById('live-room-name').textContent = name;
        document.getElementById('live-quality').textContent =
            S.chosenQuality === '0' ? 'Sin límite' : (S.chosenQuality + 'p · ' + S.chosenFps + 'fps');
        updateStatus('transmitiendo', '#10b981');

        // 3. Conectar WS y crear la sala
        connectWS();
        S.ws.onopen = () => {
            S.ws.send(JSON.stringify({
                type: 'create_room',
                name: name,
                password: password,
            }));
        };
    }
    window.startSharing = startSharing;
    document.getElementById('start-share-btn').addEventListener('click', startSharing);

