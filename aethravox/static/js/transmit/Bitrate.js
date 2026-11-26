import { S } from './state.js';

    // ====== Bitrate máximo por resolución (screen content) ======
    // Valores conservadores para no saturar la subida del emisor.
    // El emisor puede elegir "0" = sin límite.
    export function bitrateForQuality(q) {
        const map = {
            '144': 150_000,    // 150 kbps
            '240': 300_000,    // 300 kbps
            '360': 600_000,    // 600 kbps
            '480': 1_200_000,  // 1.2 Mbps
            '720': 2_500_000,  // 2.5 Mbps
            '1080': 5_000_000, // 5 Mbps
            '0': 0,            // sin límite
        };
        return map[q] || 1_200_000;
    }

    export function dimensionsForQuality(q) {
        const map = {
            '144': [256, 144],
            '240': [426, 240],
            '360': [640, 360],
            '480': [854, 480],
            '720': [1280, 720],
            '1080': [1920, 1080],
            '0': null,   // sin límite
        };
        return map[q] || null;
    }

