/* Estado compartido del módulo (era let/const top-level del IIFE). */
export const S = {
    ws: null,
    localStream: null,
    peerConnections: new Map(),
    currentRoomName: null,
    currentConnId: null,
    chosenQuality: '480',
    chosenFps: '30',
    peerDisplayMap: new Map(),
    currentControllerId: null,
    pendingRequests: new Set(),
    pointerHideTimer: null,
};
