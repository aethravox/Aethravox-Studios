import { S } from './state.js';
import { openNewChatModal } from './Modal__Nuevo_chat.js';
import { uploadImage } from './Subir_imagen.js';
import { uploadFile } from './Nota_de_voz.js';
import { sendMessage } from './Enviar_mensaje.js';

    // ====== Listeners ======
    document.getElementById('chat-new-btn').addEventListener('click', () => openNewChatModal('dm'));
    document.getElementById('chat-send').addEventListener('click', sendMessage);

    // Botón 📎 para adjuntar imagen o cualquier archivo. Un solo input file
    // acepta todo; el JS decide a qué endpoint mandarlo según el MIME.
    document.getElementById('chat-attach').addEventListener('click', () => {
        document.getElementById('chat-image-input').click();
    });
    document.getElementById('chat-image-input').addEventListener('change', async function(e) {
        const file = e.target.files[0];
        if (file) {
            if (file.type.startsWith('image/')) {
                await uploadImage(file);
            } else {
                await uploadFile(file, 'file');
            }
            e.target.value = '';  // reset para poder subir el mismo archivo otra vez
        }
    });

