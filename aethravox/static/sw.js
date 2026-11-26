/* Aethravox Studios — Service Worker
 *
 * Dos responsabilidades:
 *  1) Mostrar notificaciones push (recibidas del push service del navegador
 *     cuando la pestaña del chat está cerrada) vía el evento `push`, y abrir
 *     la conversación correcta al clickarlas vía `notificationclick`.
 *  2) Cachear assets estáticos para que la app funcione offline / cargue más
 *     rápido (PWA básico). HTML va network-first (para ver actualizaciones),
 *     assets cache-first.
 *
 * La página también puede pedir al SW que muestre una notificación in-page
 * (notificación "owned by the SW", que sobrevive un poco más que la pestaña)
 * vía postMessage({type:'show-notification', ...}).
 */

const CACHE_VERSION = 'aethravox-v6';
const CORE_ASSETS = [
    '/static/css/aethravox.css',
    '/static/fonts/inter.css',
    '/static/img/logo.png',
    '/hub',
    '/chat',
];

// ===== Install: pre-cachea assets core =====
self.addEventListener('install', (event) => {
    event.waitUntil(
        caches.open(CACHE_VERSION).then((cache) =>
            // addAll falla entero si un recurso 404; usamos add individuales
            // para que un fallo no rompa la instalación.
            Promise.all(
                CORE_ASSETS.map((url) =>
                    cache.add(url).catch((e) => console.warn('SW cache miss:', url, e))
                )
            )
        )
    );
    self.skipWaiting(); // activa el SW nuevo cuanto antes
});

// ===== Activate: limpia caches viejos + toma control =====
self.addEventListener('activate', (event) => {
    event.waitUntil(
        caches.keys().then((keys) =>
            Promise.all(
                keys
                    .filter((k) => k !== CACHE_VERSION)
                    .map((k) => caches.delete(k))
            )
        ).then(() => self.clients.claim())
    );
});

// ===== Fetch: estrategia por tipo de recurso =====
self.addEventListener('fetch', (event) => {
    const req = event.request;
    if (req.method !== 'GET') return; // solo cachea GET
    const url = new URL(req.url);

    // ===== NUNCA cachear endpoints dinámicos (API, WS, etc.) =====
    // /api/ devuelve datos que cambian en cada acción (mensajes nuevos,
    // estados de lectura, listas de conversaciones...). Si el SW los
    // cachease con cache-first, el usuario vería datos stale: p.ej. un
    // mensaje enviado no aparecería al recargar hasta borrar la caché.
    // Por eso estos van SIEMPRE a red, sin tocar la caché.
    if (url.pathname.startsWith('/api/') || url.pathname.startsWith('/chat/ws')) {
        event.respondWith(fetch(req));
        return;
    }

    // Navegación (HTML): network-first estricto. Siempre pide a red la versión
    // fresca del HTML (que enlaza el CSS/JS con el ?v= correcto); solo cae a
    // cache si no hay red. Esto evita que un HTML cacheado viejo (que enlaza
    // ?v=12) siga pidiendo CSS viejo aunque hayas subido ?v=13.
    if (req.mode === 'navigate') {
        event.respondWith(
            fetch(req)
                .then((res) => {
                    const copy = res.clone();
                    caches.open(CACHE_VERSION).then((c) => c.put(req, copy)).catch(() => {});
                    return res;
                })
                .catch(() => caches.match(req).then((r) => r || caches.match('/hub')))
        );
        return;
    }

    // Assets del propio origen: stale-while-revalidate. Sirve la cache al
    // instante (rápido) pero dispara un fetch en background para actualizarla.
    // Así, aunque un asset esté cacheado, la próxima visita ya tiene la versión
    // nueva sin necesidad de Ctrl+Shift+R. Antes era cache-first puro y el
    // usuario tenía que borrar caché a mano para ver cambios de CSS.
    if (url.origin === self.location.origin) {
        event.respondWith(
            caches.match(req).then((cached) => {
                const networkFetch = fetch(req)
                    .then((res) => {
                        if (res && res.status === 200 && res.type === 'basic') {
                            const copy = res.clone();
                            caches.open(CACHE_VERSION).then((c) => c.put(req, copy)).catch(() => {});
                        }
                        return res;
                    })
                    .catch(() => cached);
                // Si hay cache, sírvela ya; el fetch en background actualizará.
                // Si no hay cache, espera al fetch (primera carga).
                return cached || networkFetch;
            })
        );
    }
    // Recursos de otros orígenes (fuentes de Google etc.) → fetch normal.
});

// ===== Push: notificación entrante desde el push service =====
// El backend envía un payload JSON: {title, body, tag, type, conversation_id,
// url, requireInteraction}. Si el payload no es JSON válido, mostramos una
// notificación genérica con el texto plano.
self.addEventListener('push', (event) => {
    let payload = {};
    try {
        payload = event.data ? event.data.json() : {};
    } catch (e) {
        const text = event.data ? event.data.text() : '';
        payload = { title: 'Aethravox', body: text };
    }
    const title = payload.title || 'Aethravox';
    const options = {
        body: payload.body || '',
        icon: payload.icon || '/static/img/logo.png',
        badge: payload.badge || '/static/img/favicon-192.png',
        tag: payload.tag || 'aethravox',
        requireInteraction: !!payload.requireInteraction,
        data: {
            url: payload.url || '/chat',
            conversation_id: payload.conversation_id || null,
            type: payload.type || 'message',
        },
    };
    event.waitUntil(self.registration.showNotification(title, options));
});

// ===== Notificationclick: abrir/focusar la pestaña en la URL correcta =====
self.addEventListener('notificationclick', (event) => {
    event.notification.close();
    const targetUrl = (event.notification.data && event.notification.data.url) || '/chat';
    event.waitUntil(
        (async () => {
            const allClients = await self.clients.matchAll({
                type: 'window',
                includeUncontrolled: true,
            });
            // 1) Si hay una pestaña en la URL destino (o que la incluya), focusearla.
            for (const client of allClients) {
                if (client.url && client.url.includes(targetUrl)) {
                    client.focus();
                    // Avisar a la pestaña para que abra la conversación si procede.
                    if (event.notification.data && event.notification.data.conversation_id) {
                        client.postMessage({
                            type: 'notification-clicked',
                            conversation_id: event.notification.data.conversation_id,
                            notifType: event.notification.data.type,
                        });
                    }
                    return;
                }
            }
            // 2) Si no hay pestaña en esa URL pero hay alguna abierta, focusearla.
            if (allClients.length > 0) {
                allClients[0].focus();
                return;
            }
            // 3) Sin pestañas abiertas: abrir una nueva en la URL destino.
            await self.clients.openWindow(targetUrl);
        })()
    );
});

// ===== Message: la página pide al SW que muestre una notificación =====
// Así las notificaciones in-page son "owned by the SW" (sobreviven un poco
// más que la pestaña y centralizan el click). Formato:
//   postMessage({ type:'show-notification', title, body, tag, icon,
//                 requireInteraction, url, conversation_id })
self.addEventListener('message', (event) => {
    const data = event.data;
    if (!data || data.type !== 'show-notification') return;
    const title = data.title || 'Aethravox';
    const options = {
        body: data.body || '',
        icon: data.icon || '/static/img/logo.png',
        tag: data.tag || 'aethravox',
        requireInteraction: !!data.requireInteraction,
        data: {
            url: data.url || '/chat',
            conversation_id: data.conversation_id || null,
            type: data.type || 'message',
        },
    };
    event.waitUntil(self.registration.showNotification(title, options));
});
