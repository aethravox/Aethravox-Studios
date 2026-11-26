// ============ STATE ============
const G = {
  mode: 'menu', score: 0, combo: 0, maxCombo: 0, brushColor: '#00ffff', brushSize: 5, brushOpacity: 1,
  brushMode: 'glow', drawing: false, lastX: 0, lastY: 0, history: [], redoStack: [], maxHistory: 30,
  particles: [], bgParticles: [], challengeIdx: 0, challengeTimer: 0, challengeActive: false,
  challengesCompleted: 0, totalDrawPixels: 0, symmetry: 1, soundOn: true, gridOn: false,
  powerups: { rainbow: 3, shield: 2, bomb: 1, mirror: 2, time: 2, freeze: 1 },
  activePowerups: { rainbow: false, shield: false, mirror: false, freeze: false },
  rainbowHue: 0, mirrorOn: false, zenBreathPhase: 0, strokeCount: 0, comboTimer: 0,
  achievements: {}, totalStrokes: 0, bestScore: 0,
  vsMode: false, vsOpponentScore: 0, vsOpponentDrawing: [], vsTimer: 0, vsSpeed: 1,
  mpMode: false, mpIsHost: false, mpPlayers: new Map(), mpWs: null, mpPlayerId: 0,
  mpServerPort: 8765, mpConnected: false, cwDrag: false, _lastT: -1
};

// ============ CHALLENGES ============
const CATS = {
  animales: ['perro','gato','pez','ave','serpiente','caballo','vaca','cerdo','oveja','conejo','tortuga','mariposa','abeja','araña','león','tigre','elefante','jirafa','mono','delfín','tiburón','pulpo','cangrejo','águila','búho','pingüino','loro','camaleón','cocodrilo','rana'],
  objetos: ['casa','coche','bicicleta','reloj','teléfono','vela','llave','sombrero','zapato','gafas','paraguas','libro','taza','silla','mesa','puerta','ventana','escalera','espada','escudo','corona','anillo','guitarra','batería','cámara','televisor','computadora','lápiz','pincel','tijeras'],
  naturaleza: ['árbol','flor','montaña','río','nube','sol','luna','estrella','arcoíris','cascada','volcán','isla','desierto','bosque','playa','océano','lago','cueva','glaciar','aurora boreal','tornado','trueno','nieve','lluvia','roca','cristal','diamante','seta','hoja','cereza'],
  fantástico: ['dragón','unicornio','fénix','hada','duende','bruja','mago','caballero','princesa','pirata','vampiro','hombre lobo','zombi','alien','monstruo','sirena','centauro','pegaso','grifo','ángel'],
  simbolos: ['corazón','infinito','paz','yin yang','espiral','círculo','triángulo','cuadrado','estrella de 5 puntas','rayo','llama','copo de nieve','nota musical','interrogación','sol radiante']
};

let ALL_C = [];
Object.entries(CATS).forEach(([c, items]) => {
  items.forEach(i => {
    ALL_C.push({ name: 'Dibuja: ' + i, hint: c.toUpperCase(), time: 45 + Math.floor(Math.random() * 30) });
  });
});
function shuffleC() {
  for (let i = ALL_C.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [ALL_C[i], ALL_C[j]] = [ALL_C[j], ALL_C[i]];
  }
}

// ============ ACHIEVEMENTS ============
const ACHS = [
  { id: 'first', name: 'Primer Trazo', desc: 'Dibuja por primera vez', check: () => G.totalStrokes >= 1 },
  { id: 'hundred', name: 'Artista', desc: '100 trazos realizados', check: () => G.totalStrokes >= 100 },
  { id: 'thousand', name: 'Maestro del Neón', desc: '1000 trazos realizados', check: () => G.totalStrokes >= 1000 },
  { id: 'five_w', name: 'Racha x5', desc: 'Completa 5 desafíos', check: () => G.challengesCompleted >= 5 },
  { id: 'score5', name: '500 Puntos', desc: 'Alcanza 500 pts', check: () => G.score >= 500 },
  { id: 'combo5', name: 'Combo x5', desc: 'Mantén un combo de x5', check: () => G.maxCombo >= 5 },
  { id: 'vs_win', name: 'Campeón', desc: 'Gana contra la IA', check: () => G.achievements.vs_win },
  { id: 'kaleido', name: 'Psicodélico', desc: 'Usa la herramienta Caleidoscopio', check: () => G.achievements.kaleido }
];
function checkAch() {
  ACHS.forEach(a => {
    if (!G.achievements[a.id] && a.check()) {
      G.achievements[a.id] = true;
      showAch(a.name);
      playS('achievement');
    }
  });
}
function showAch(n) {
  const el = document.getElementById('achievementPopup');
  el.textContent = '¡LOGRO DESBLOQUEADO: ' + n + '!';
  el.classList.add('show');
  setTimeout(() => el.classList.remove('show'), 2500);
}

// ============ SOUND (Web Audio API) ============
let actx;
function initA() {
  if (!actx) actx = new (window.AudioContext || window.webkitAudioContext)();
}
function playS(t) {
  if (!G.soundOn) return;
  try { initA(); } catch (e) { return; }
  const n = actx.currentTime;
  const o = actx.createOscillator();
  const g = actx.createGain();
  o.connect(g);
  g.connect(actx.destination);
  switch (t) {
    case 'draw': o.frequency.value = 220 + Math.min(G.lastY, 600); o.type = 'sine'; g.gain.value = 0.02; break;
    case 'color': o.frequency.value = 587.33; o.type = 'triangle'; g.gain.value = 0.05; break;
    case 'combo': o.frequency.value = 700 + G.combo * 50; o.type = 'sine'; g.gain.value = 0.06; break;
    case 'achievement': o.frequency.value = 523.25; o.type = 'square'; g.gain.value = 0.07; break;
    case 'timer': o.frequency.value = 440; o.type = 'sawtooth'; g.gain.value = 0.03; break;
    case 'boom': o.frequency.value = 90; o.type = 'sawtooth'; g.gain.value = 0.12; break;
    case 'win': o.frequency.value = 659.25; o.type = 'sine'; g.gain.value = 0.07; break;
    case 'lose': o.frequency.value = 180; o.type = 'sawtooth'; g.gain.value = 0.07; break;
    case 'start': o.frequency.value = 440; o.type = 'triangle'; g.gain.value = 0.05; break;
    default: o.frequency.value = 300; o.type = 'sine'; g.gain.value = 0.02;
  }
  g.gain.exponentialRampToValueAtTime(0.001, n + 0.25);
  o.start(n);
  o.stop(n + 0.25);
}

// ============ CANVASES & RESIZE ============
const bgC = document.getElementById('bgCanvas'), drC = document.getElementById('drawCanvas'), ptC = document.getElementById('particleCanvas');
const bgX = bgC.getContext('2d'), drX = drC.getContext('2d'), ptX = ptC.getContext('2d');

function resize() {
  const w = window.innerWidth, h = window.innerHeight;
  [bgC, drC, ptC].forEach(c => {
    c.width = w;
    c.height = h;
  });
  initBg();
}
window.addEventListener('resize', resize);

// ============ BACKGROUND PARTICLES ============
function initBg() {
  G.bgParticles = [];
  const n = Math.floor((bgC.width * bgC.height) / 14000);
  for (let i = 0; i < n; i++) {
    G.bgParticles.push({
      x: Math.random() * bgC.width,
      y: Math.random() * bgC.height,
      vx: (Math.random() - 0.5) * 0.3,
      vy: (Math.random() - 0.5) * 0.3,
      r: Math.random() * 1.5 + 0.5,
      a: Math.random() * 0.3 + 0.1,
      h: Math.random() * 60 + 170
    });
  }
}
function drawBg() {
  bgX.fillStyle = 'rgba(10,10,15,0.18)';
  bgX.fillRect(0, 0, bgC.width, bgC.height);
  G.bgParticles.forEach(p => {
    p.x += p.vx; p.y += p.vy;
    if (p.x < 0) p.x = bgC.width;
    if (p.x > bgC.width) p.x = 0;
    if (p.y < 0) p.y = bgC.height;
    if (p.y > bgC.height) p.y = 0;
    bgX.beginPath();
    bgX.arc(p.x, p.y, p.r, 0, Math.PI * 2);
    bgX.fillStyle = `hsla(${p.h},100%,70%,${p.a})`;
    bgX.fill();
  });
}

// ============ PARTICLES ============
function spawnP(x, y, c, n) {
  for (let i = 0; i < n; i++) {
    const a = Math.random() * Math.PI * 2, s = Math.random() * 3 + 1;
    G.particles.push({
      x, y, vx: Math.cos(a) * s, vy: Math.sin(a) * s, life: 1, decay: Math.random() * 0.03 + 0.02, r: Math.random() * 3 + 1, color: c
    });
  }
}
function drawP() {
  ptX.clearRect(0, 0, ptC.width, ptC.height);
  for (let i = G.particles.length - 1; i >= 0; i--) {
    const p = G.particles[i];
    p.x += p.vx; p.y += p.vy; p.vy += 0.04; p.life -= p.decay;
    if (p.life <= 0) { G.particles.splice(i, 1); continue; }
    ptX.beginPath();
    ptX.arc(p.x, p.y, p.r * p.life, 0, Math.PI * 2);
    let r = 0, g = 255, b = 255;
    if (p.color.startsWith('#') && p.color.length === 7) {
      r = parseInt(p.color.slice(1, 3), 16);
      g = parseInt(p.color.slice(3, 5), 16);
      b = parseInt(p.color.slice(5, 7), 16);
    }
    ptX.fillStyle = `rgba(${r},${g},${b},${p.life})`;
    ptX.shadowBlur = 6;
    ptX.shadowColor = p.color;
    ptX.fill();
  }
  ptX.shadowBlur = 0;
}

// ============ COLOR WHEEL ============
const cwC = document.getElementById('cwCanvas'), cwX = cwC.getContext('2d');
let cwH = 180, cwS = 100, cwL = 50;
function drawCW() {
  const cx = 90, cy = 90, r = 80;
  for (let a = 0; a < 360; a++) {
    const r1 = (a - 1) * Math.PI / 180, r2 = (a + 1) * Math.PI / 180;
    cwX.beginPath(); cwX.moveTo(cx, cy); cwX.arc(cx, cy, r, r1, r2); cwX.closePath();
    cwX.fillStyle = `hsl(${a},100%,50%)`; cwX.fill();
  }
  cwX.beginPath(); cwX.arc(cx, cy, r - 22, 0, Math.PI * 2); cwX.fillStyle = '#111'; cwX.fill();
  const sg = cwX.createRadialGradient(cx, cy, 0, cx, cy, r - 25);
  sg.addColorStop(0, '#fff'); sg.addColorStop(1, `hsl(${cwH},100%,50%)`);
  cwX.beginPath(); cwX.arc(cx, cy, r - 25, 0, Math.PI * 2); cwX.fillStyle = sg; cwX.fill();
}
function updateCW() {
  const c = `hsl(${cwH},${cwS}%,${cwL}%)`;
  const t = document.createElement('canvas').getContext('2d');
  t.fillStyle = c;
  const hex = t.fillStyle;
  G.brushColor = hex;
  document.getElementById('cwPreview').style.background = hex;
  document.getElementById('cwHex').textContent = hex;
}
function setupPresets() {
  const ps = ['#ff0000','#ff8800','#ffff00','#00ff00','#00ffff','#0088ff','#8800ff','#ff00ff','#ffffff','#000000'];
  const row = document.getElementById('cwPresets'); row.innerHTML = '';
  ps.forEach(c => {
    const d = document.createElement('div'); d.className = 'cw-p'; d.style.background = c;
    d.onclick = () => { G.brushColor = c; document.getElementById('cwPreview').style.background = c; document.getElementById('cwHex').textContent = c; playS('color'); };
    row.appendChild(d);
  });
}
drawCW(); updateCW(); setupPresets();

cwC.addEventListener('mousedown', pickCW);
cwC.addEventListener('mousemove', e => { if (e.buttons === 1) pickCW(e); });
cwC.addEventListener('touchstart', pickCW);
cwC.addEventListener('touchmove', pickCW);
function pickCW(e) {
  const rect = cwC.getBoundingClientRect();
  const t = e.touches ? e.touches[0] : e;
  const x = t.clientX - rect.left - 90, y = t.clientY - rect.top - 90;
  const d = Math.sqrt(x * x + y * y);
  if (d > 55 && d < 82) {
    cwH = Math.atan2(y, x) * 180 / Math.PI;
    if (cwH < 0) cwH += 360;
  } else if (d <= 55) {
    cwS = Math.min(100, Math.floor((d / 55) * 100));
    cwL = Math.max(10, Math.min(90, 50 - (y / 55) * 50));
  }
  drawCW(); updateCW(); playS('color');
}

document.getElementById('cwOpacity').addEventListener('input', e => {
  G.brushOpacity = parseInt(e.target.value) / 100;
  document.getElementById('opacityBtn').textContent = G.brushOpacity.toFixed(1);
});
document.getElementById('cwSizeR').addEventListener('input', e => {
  G.brushSize = parseInt(e.target.value);
  document.getElementById('sizeSlider').value = G.brushSize;
  document.getElementById('sizeVal').textContent = G.brushSize;
});
document.getElementById('sizeSlider').addEventListener('input', e => {
  G.brushSize = parseInt(e.target.value);
  document.getElementById('cwSizeR').value = G.brushSize;
  document.getElementById('sizeVal').textContent = G.brushSize;
});

// ============ BRUSH LOGIC ============
function h2r(hex, a) {
  if (!hex || !hex.startsWith('#') || hex.length !== 7) return `rgba(0,255,255,${a})`;
  const r = parseInt(hex.slice(1, 3), 16), g = parseInt(hex.slice(3, 5), 16), b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r},${g},${b},${a})`;
}
function getCol() { return G.activePowerups.rainbow ? `hsl(${G.rainbowHue},100%,60%)` : G.brushColor; }

function drawBrush(ctx, x, y, lx, ly) {
  const color = getCol(), size = G.brushSize, op = G.brushOpacity;
  ctx.globalAlpha = op;
  for (let s = 0; s < G.symmetry; s++) {
    const angle = (Math.PI * 2 / G.symmetry) * s;
    ctx.save();
    ctx.translate(drC.width / 2, drC.height / 2);
    ctx.rotate(angle);
    ctx.translate(-drC.width / 2, -drC.height / 2);
    drawB1(ctx, x, y, lx, ly, color, size);
    if (G.mirrorOn && G.brushMode !== 'eraser' && G.brushMode !== 'fill') {
      drawB1(ctx, drC.width - x, y, drC.width - lx, ly, color, size);
    }
    ctx.restore();
  }
  if (G.brushMode === 'kaleidoscope') G.achievements.kaleido = true;
  G.totalDrawPixels += Math.abs(x - lx) + Math.abs(y - ly);
  G.rainbowHue = (G.rainbowHue + 3) % 360;
  ctx.globalAlpha = 1;
}

function drawB1(ctx, x, y, lx, ly, color, size) {
  switch (G.brushMode) {
    case 'brush':
      ctx.beginPath(); ctx.moveTo(lx, ly); ctx.lineTo(x, y);
      ctx.strokeStyle = color; ctx.lineWidth = size; ctx.lineCap = 'round';
      ctx.shadowBlur = size * 1.5; ctx.shadowColor = color; ctx.stroke(); ctx.shadowBlur = 0;
      spawnP(x, y, color, 1);
      break;
    case 'spray':
      for (let i = 0; i < size * 2; i++) {
        const a = Math.random() * Math.PI * 2, r = Math.random() * size * 2;
        ctx.fillStyle = h2r(color, Math.random() * 0.6 + 0.3);
        ctx.fillRect(x + Math.cos(a) * r, y + Math.sin(a) * r, 2, 2);
      }
      spawnP(x, y, color, 1);
      break;
    case 'glow':
      for (let i = 3; i >= 0; i--) {
        ctx.beginPath(); ctx.moveTo(lx, ly); ctx.lineTo(x, y);
        ctx.strokeStyle = h2r(color, 0.15 * (4 - i));
        ctx.lineWidth = size + ((3 - i) * 6); ctx.lineCap = 'round'; ctx.stroke();
      }
      ctx.beginPath(); ctx.moveTo(lx, ly); ctx.lineTo(x, y);
      ctx.strokeStyle = '#fff'; ctx.lineWidth = Math.max(1, size / 3); ctx.lineCap = 'round'; ctx.stroke();
      spawnP(x, y, color, 2);
      break;
    case 'ribbon':
      const w = Math.sin(Date.now() / 100) * size * 2;
      ctx.beginPath(); ctx.moveTo(lx + w, ly - w); ctx.quadraticCurveTo((lx + x) / 2 + w * 2, (ly + y) / 2 - w * 2, x + w, y - w);
      ctx.strokeStyle = color; ctx.lineWidth = size * 0.6; ctx.lineCap = 'round';
      ctx.shadowBlur = 10; ctx.shadowColor = color; ctx.stroke(); ctx.shadowBlur = 0;
      break;
    case 'star':
      if (Math.random() > 0.4) drawStar5(ctx, x + (Math.random() - 0.5) * size * 3, y + (Math.random() - 0.5) * size * 3, 5, size * 1.4, size * 0.6, color);
      ctx.beginPath(); ctx.moveTo(lx, ly); ctx.lineTo(x, y); ctx.strokeStyle = h2r(color, 0.3); ctx.lineWidth = 1; ctx.stroke();
      spawnP(x, y, color, 1);
      break;
    case 'calligraphy':
      const sp = Math.sqrt((x - lx) ** 2 + (y - ly) ** 2);
      ctx.beginPath(); ctx.moveTo(lx, ly); ctx.lineTo(x, y); ctx.strokeStyle = color;
      ctx.lineWidth = Math.max(1, size * 2 - sp * 0.4); ctx.lineCap = 'round';
      ctx.shadowBlur = 4; ctx.shadowColor = color; ctx.stroke(); ctx.shadowBlur = 0;
      break;
    case 'pixel':
      const gs = Math.max(3, size), px = Math.floor(x / gs) * gs, py = Math.floor(y / gs) * gs;
      ctx.fillStyle = color; ctx.fillRect(px, py, gs, gs);
      break;
    case 'kaleidoscope':
      for (let k = 0; k < 6; k++) {
        const ka = k * Math.PI / 3;
        ctx.beginPath(); ctx.arc(x + Math.cos(ka) * size * 2, y + Math.sin(ka) * size * 2, size * 0.5, 0, Math.PI * 2);
        ctx.fillStyle = h2r(color, 0.6); ctx.fill();
      }
      spawnP(x, y, color, 3);
      break;
    case 'fire':
      for (let i = 0; i < 4; i++) {
        const fx = x + (Math.random() - 0.5) * size * 2, fy = y - Math.random() * size * 2.5, fr = Math.random() * size * 0.8 + 1;
        ctx.beginPath(); ctx.arc(fx, fy, fr, 0, Math.PI * 2); ctx.fillStyle = `hsla(${Math.random() * 40},100%,60%,${Math.random() * 0.5 + 0.3})`; ctx.fill();
      }
      spawnP(x, y, '#ff4400', 2);
      break;
    case 'galaxy':
      for (let i = 0; i < 6; i++) {
        const ga = Math.random() * Math.PI * 2, gd = Math.random() * size * 2.5;
        ctx.beginPath(); ctx.arc(x + Math.cos(ga) * gd, y + Math.sin(ga) * gd, Math.random() * 2 + 0.5, 0, Math.PI * 2);
        ctx.fillStyle = h2r(['#fff', '#0ff', '#f0f', '#ff0'][Math.floor(Math.random() * 4)], Math.random() * 0.8 + 0.2); ctx.fill();
      }
      ctx.beginPath(); ctx.arc(x, y, size * 0.7, 0, Math.PI * 2); ctx.fillStyle = h2r(color, 0.2); ctx.fill();
      break;
    case 'lightning':
      ctx.beginPath(); ctx.moveTo(lx, ly);
      let cx2 = lx, cy2 = ly;
      for (let i = 0; i < 4; i++) {
        cx2 += (x - lx) / 4 + (Math.random() - 0.5) * size * 2;
        cy2 += (y - ly) / 4 + (Math.random() - 0.5) * size * 2;
        ctx.lineTo(cx2, cy2);
      }
      ctx.lineTo(x, y); ctx.strokeStyle = color; ctx.lineWidth = Math.max(1, size * 0.5); ctx.lineCap = 'round';
      ctx.shadowBlur = 12; ctx.shadowColor = color; ctx.stroke(); ctx.shadowBlur = 0;
      break;
    case 'bubble':
      const br = size + Math.random() * size;
      ctx.beginPath(); ctx.arc(x, y, br, 0, Math.PI * 2); ctx.strokeStyle = h2r(color, 0.6); ctx.lineWidth = 2; ctx.stroke();
      ctx.beginPath(); ctx.arc(x - br * 0.3, y - br * 0.3, br * 0.2, 0, Math.PI * 2); ctx.fillStyle = 'rgba(255,255,255,0.4)'; ctx.fill();
      break;
    case 'gravity':
      spawnP(x, y, color, 2);
      for (let i = 0; i < Math.min(2, G.particles.length); i++) {
        const p = G.particles[G.particles.length - 1 - i];
        if (p) { p.vy = -2; p.r = Math.random() * 4 + 2; }
      }
      break;
    case 'dna':
      const dt = Date.now() / 150;
      ctx.beginPath(); ctx.arc(x + Math.sin(dt) * size * 2, y - Math.cos(dt) * size * 2, size * 0.4, 0, Math.PI * 2); ctx.fillStyle = h2r(color, 0.8); ctx.fill();
      ctx.beginPath(); ctx.arc(x - Math.sin(dt) * size * 2, y + Math.cos(dt) * size * 2, size * 0.4, 0, Math.PI * 2); ctx.fillStyle = h2r('#ff00ff', 0.8); ctx.fill();
      break;
    case 'fractal':
      drawFrac(ctx, x, y, size * 2.5, Math.atan2(y - ly, x - lx) + Math.PI / 2, color, 3);
      break;
    case 'mosaic':
      const ms = Math.max(6, size * 2), mx = Math.floor(x / ms) * ms, my = Math.floor(y / ms) * ms;
      ctx.fillStyle = ['#f00', '#0f0', '#00f', '#ff0', '#f0f', '#0ff'][Math.floor(Math.random() * 6)];
      ctx.fillRect(mx, my, ms, ms); ctx.strokeStyle = 'rgba(0,0,0,0.4)'; ctx.lineWidth = 1; ctx.strokeRect(mx, my, ms, ms);
      break;
    case 'wool':
      ctx.beginPath(); ctx.moveTo(lx, ly);
      for (let w = 0; w < 4; w++) ctx.quadraticCurveTo(x + (Math.random() - 0.5) * size * 2.5, y + (Math.random() - 0.5) * size * 2.5, x, y);
      ctx.strokeStyle = h2r(color, 0.4); ctx.lineWidth = size * 0.35; ctx.stroke();
      break;
    case 'neonpulse':
      const pu = Math.sin(Date.now() / 60) * 0.5 + 0.5;
      ctx.beginPath(); ctx.moveTo(lx, ly); ctx.lineTo(x, y);
      ctx.strokeStyle = color; ctx.lineWidth = size * pu * 2.2; ctx.lineCap = 'round';
      ctx.shadowBlur = size * 3 * pu; ctx.shadowColor = color; ctx.stroke(); ctx.shadowBlur = 0;
      break;
    case 'eraser':
      ctx.globalCompositeOperation = 'destination-out';
      ctx.beginPath(); ctx.moveTo(lx, ly); ctx.lineTo(x, y);
      ctx.strokeStyle = 'rgba(0,0,0,1)'; ctx.lineWidth = size * 3; ctx.lineCap = 'round'; ctx.stroke();
      ctx.globalCompositeOperation = 'source-over';
      break;
    case 'fill':
      floodFill(Math.floor(x), Math.floor(y), color);
      break;
  }
}

function drawStar5(ctx, cx, cy, sp, oR, iR, c) {
  let rot = Math.PI / 2 * 3, step = Math.PI / sp;
  ctx.beginPath(); ctx.moveTo(cx, cy - oR);
  for (let i = 0; i < sp; i++) {
    ctx.lineTo(cx + Math.cos(rot) * oR, cy + Math.sin(rot) * oR); rot += step;
    ctx.lineTo(cx + Math.cos(rot) * iR, cy + Math.sin(rot) * iR); rot += step;
  }
  ctx.closePath(); ctx.fillStyle = c; ctx.shadowBlur = 8; ctx.shadowColor = c; ctx.fill(); ctx.shadowBlur = 0;
}
function drawFrac(ctx, x, y, l, a, c, d) {
  if (d <= 0 || l < 2) return;
  const x2 = x + Math.cos(a) * l, y2 = y + Math.sin(a) * l;
  ctx.beginPath(); ctx.moveTo(x, y); ctx.lineTo(x2, y2);
  ctx.strokeStyle = h2r(c, 0.4 + d * 0.15); ctx.lineWidth = Math.max(1, d); ctx.stroke();
  drawFrac(ctx, x2, y2, l * 0.65, a - 0.4, c, d - 1);
  drawFrac(ctx, x2, y2, l * 0.65, a + 0.4, c, d - 1);
}
function floodFill(sx, sy, fc) {
  const w = drC.width, h = drC.height, id = drX.getImageData(0, 0, w, h), d = id.data;
  const idx = (sy * w + sx) * 4;
  const tr = d[idx], tg = d[idx + 1], tb = d[idx + 2];
  const t = document.createElement('canvas').getContext('2d');
  t.fillStyle = fc; t.fillRect(0, 0, 1, 1);
  const f = t.getImageData(0, 0, 1, 1).data;
  if (tr === f[0] && tg === f[1] && tb === f[2]) return;
  const st = [[sx, sy]]; let cnt = 0; const mx = w * h / 4;
  while (st.length > 0 && cnt < mx) {
    const [cx, cy] = st.pop(); const ci = (cy * w + cx) * 4;
    if (cx < 0 || cx >= w || cy < 0 || cy >= h) continue;
    if (Math.abs(d[ci] - tr) > 30 || Math.abs(d[ci + 1] - tg) > 30 || Math.abs(d[ci + 2] - tb) > 30) continue;
    d[ci] = f[0]; d[ci + 1] = f[1]; d[ci + 2] = f[2]; d[ci + 3] = 255;
    cnt++;
    st.push([cx + 1, cy], [cx - 1, cy], [cx, cy + 1], [cx, cy - 1]);
  }
  drX.putImageData(id, 0, 0);
}

// ============ UNDO / REDO / GRID ============
function saveState() {
  if (G.history.length >= G.maxHistory) G.history.shift();
  G.history.push(drX.getImageData(0, 0, drC.width, drC.height));
  G.redoStack = [];
}
function undo() {
  if (G.history.length > 0) {
    G.redoStack.push(drX.getImageData(0, 0, drC.width, drC.height));
    drX.putImageData(G.history.pop(), 0, 0);
    playS('color');
  }
}
function redo() {
  if (G.redoStack.length > 0) {
    G.history.push(drX.getImageData(0, 0, drC.width, drC.height));
    drX.putImageData(G.redoStack.pop(), 0, 0);
    playS('color');
  }
}
function drawGrid() {
  if (!G.gridOn) return;
  drX.save(); drX.globalAlpha = 0.08; drX.strokeStyle = '#0ff'; drX.lineWidth = 0.5;
  const gs = 30;
  for (let x = 0; x < drC.width; x += gs) { drX.beginPath(); drX.moveTo(x, 0); drX.lineTo(x, drC.height); drX.stroke(); }
  for (let y = 0; y < drC.height; y += gs) { drX.beginPath(); drX.moveTo(0, y); drX.lineTo(drC.width, y); drX.stroke(); }
  drX.restore();
}

// ============ SCORING ============
function addScore(p) {
  G.comboTimer = 60; G.combo++;
  if (G.combo > G.maxCombo) G.maxCombo = G.combo;
  G.score += p * Math.min(G.combo, 10);
  updateUI(); showCombo();
  if (G.combo % 5 === 0) playS('combo');
}
function showCombo() {
  const el = document.getElementById('comboDisplay');
  el.textContent = `x${Math.min(G.combo, 10)}`;
  el.style.opacity = '1';
  el.style.transform = 'scale(1.2)';
  setTimeout(() => el.style.transform = 'scale(1)', 150);
}
function updateUI() { document.getElementById('scoreDisplay').textContent = G.score + ' pts'; }

// ============ CONFIRM DIALOG ============
function showConfirm(title, msg, onYes) {
  document.getElementById('confirmTitle').textContent = title;
  document.getElementById('confirmMsg').textContent = msg;
  document.getElementById('confirmDialog').style.display = 'block';
  document.getElementById('confirmYes').onclick = () => { document.getElementById('confirmDialog').style.display = 'none'; onYes(); };
  document.getElementById('confirmNo').onclick = () => { document.getElementById('confirmDialog').style.display = 'none'; };
}

// ============ MODES & CHALLENGES ============
function startCh() {
  G.mode = 'challenge'; G.score = 0; G.combo = 0; G.maxCombo = 0; G.totalDrawPixels = 0; G.challengesCompleted = 0;
  G.powerups = { rainbow: 3, shield: 2, bomb: 1, mirror: 2, time: 2, freeze: 1 }; updatePW();
  shuffleC(); G.challengeIdx = 0;
  drX.clearRect(0, 0, drC.width, drC.height); G.history = []; G.redoStack = [];
  nextCh(); hide('menuScreen'); hide('resultsScreen'); playS('start');
}
function nextCh() {
  if (G.challengeIdx >= ALL_C.length) G.challengeIdx = 0;
  const ch = ALL_C[G.challengeIdx];
  G.challengeActive = true; G.challengeTimer = ch.time; G.challengeIdx++; G.challengesCompleted++;
  document.getElementById('challengeText').textContent = ch.name;
  document.getElementById('timerDisplay').textContent = ch.time + 's';
  document.getElementById('timerDisplay').style.color = '#0f0';
  showOv(ch.name, ch.hint + ' - ' + ch.time + 's'); saveState();
}
function showOv(t, d) {
  const ol = document.getElementById('challengeOverlay');
  document.getElementById('overlayTitle').textContent = t;
  document.getElementById('overlayDesc').textContent = d;
  ol.classList.add('show');
  setTimeout(() => ol.classList.remove('show'), 2000);
}
function chTick() {
  if (!G.challengeActive) return;
  if (!G.activePowerups.freeze) G.challengeTimer -= 1 / 60;
  const t = Math.ceil(G.challengeTimer);
  document.getElementById('timerDisplay').textContent = t + 's';
  if (t <= 5) {
    document.getElementById('timerDisplay').style.color = '#f00';
    if (t !== G._lastT) { playS('timer'); G._lastT = t; }
  } else if (t <= 10) document.getElementById('timerDisplay').style.color = '#ff0';
  
  if (G.challengeTimer <= 0) {
    G.challengeActive = false;
    addScore(Math.floor(G.totalDrawPixels / 10));
    if (G.challengesCompleted >= 5) showRes();
    else setTimeout(() => { drX.clearRect(0, 0, drC.width, drC.height); nextCh(); }, 1500);
  }
}
function showRes() {
  if (G.vsMode) { endVS(); return; }
  document.getElementById('resultsScreen').style.display = 'flex';
  document.getElementById('finalScore').textContent = G.score;
  document.getElementById('finalCombo').textContent = G.maxCombo;
  document.getElementById('finalAcc').textContent = Math.min(100, Math.floor(G.totalDrawPixels / 40)) + '%';
  document.getElementById('finalTime').textContent = (5 * 50 - Math.ceil(G.challengeTimer)) + 's';
  document.getElementById('vsResult').style.display = 'none';
  if (G.score > G.bestScore) G.bestScore = G.score;
  checkAch(); playS('win');
}
function hide(id) { document.getElementById(id).style.display = 'none'; }
function show(id, v) { document.getElementById(id).style.display = v || 'block'; }

// ============ VS AI MODE ============
let vsOC, vsOX;
function startVS() {
  G.vsMode = true; G.mode = 'challenge'; G.score = 0; G.combo = 0; G.maxCombo = 0; G.totalDrawPixels = 0; G.challengesCompleted = 0;
  G.powerups = { rainbow: 3, shield: 2, bomb: 1, mirror: 2, time: 2, freeze: 1 }; updatePW();
  G.vsOpponentScore = 0; G.vsOpponentDrawing = []; G.vsTimer = 0; G.vsSpeed = 1;
  shuffleC(); G.challengeIdx = 0;
  drX.clearRect(0, 0, drC.width, drC.height); G.history = []; G.redoStack = [];
  
  const box = document.getElementById('oppBox'); box.style.display = 'block'; box.innerHTML = '';
  const card = document.createElement('div'); card.className = 'opp-card';
  const cv = document.createElement('canvas'); cv.width = 85; cv.height = 105; cv.id = 'oppCV';
  const nm = document.createElement('div'); nm.className = 'opp-name'; nm.textContent = 'IA Rival';
  const sc = document.createElement('div'); sc.className = 'opp-score'; sc.id = 'oppScore'; sc.textContent = '0 pts';
  card.appendChild(cv); card.appendChild(nm); card.appendChild(sc); box.appendChild(card);
  vsOC = document.getElementById('oppCV'); vsOX = vsOC.getContext('2d');
  
  nextCh(); hide('menuScreen'); hide('resultsScreen'); playS('start');
}
function aiTick() {
  if (!G.vsMode || !G.challengeActive) return;
  G.vsTimer++;
  if (G.vsTimer % 12 === 0) {
    aiDrawShape(); G.vsOpponentScore += Math.floor(Math.random() * 15 * G.vsSpeed) + 5;
    if (G.vsTimer % 300 === 0) G.vsSpeed = Math.min(4, G.vsSpeed + 0.25);
  }
  if (G.vsTimer % 6 === 0 && vsOX) updateOppPreview();
  const sc = document.getElementById('oppScore'); if (sc) sc.textContent = G.vsOpponentScore + ' pts';
}
function aiDrawShape() {
  const shapes = ['circle', 'rect', 'line', 'triangle', 'arc'];
  const shape = shapes[Math.floor(Math.random() * shapes.length)];
  const colors = ['#0ff', '#f0f', '#0f0', '#ff0', '#f60', '#f00', '#88f'];
  G.vsOpponentDrawing.push({
    type: shape, x: Math.random() * drC.width, y: Math.random() * drC.height, r: Math.random() * 30 + 10,
    color: colors[Math.floor(Math.random() * colors.length)], x2: Math.random() * drC.width, y2: Math.random() * drC.height
  });
}
function updateOppPreview() {
  if (!vsOX) return;
  vsOX.fillStyle = 'rgba(10,10,15,0.15)'; vsOX.fillRect(0, 0, 85, 105);
  G.vsOpponentDrawing.slice(-25).forEach(d => {
    const sx = (d.x / drC.width) * 85, sy = (d.y / drC.height) * 105, sr = Math.max(2, (d.r / drC.width) * 85);
    vsOX.fillStyle = d.color; vsOX.strokeStyle = d.color; vsOX.lineWidth = 1.5;
    if (d.type === 'circle') { vsOX.beginPath(); vsOX.arc(sx, sy, sr, 0, Math.PI * 2); vsOX.fill(); }
    else if (d.type === 'rect') vsOX.fillRect(sx - sr, sy - sr, sr * 2, sr * 2);
    else if (d.type === 'line') { vsOX.beginPath(); vsOX.moveTo(sx, sy); vsOX.lineTo((d.x2 / drC.width) * 85, (d.y2 / drC.height) * 105); vsOX.stroke(); }
    else { vsOX.beginPath(); vsOX.arc(sx, sy, sr, 0, Math.PI); vsOX.stroke(); }
  });
}
function endVS() {
  document.getElementById('oppBox').style.display = 'none';
  document.getElementById('resultsScreen').style.display = 'flex';
  document.getElementById('finalScore').textContent = G.score;
  document.getElementById('finalCombo').textContent = G.maxCombo;
  document.getElementById('finalAcc').textContent = Math.min(100, Math.floor(G.totalDrawPixels / 40)) + '%';
  document.getElementById('finalTime').textContent = (5 * 50 - Math.ceil(G.challengeTimer)) + 's';
  const vr = document.getElementById('vsResult'); vr.style.display = 'block';
  if (G.score > G.vsOpponentScore) {
    vr.textContent = '¡VICTORIA! Tu: ' + G.score + ' vs IA: ' + G.vsOpponentScore;
    vr.style.color = '#0f0'; playS('win'); G.achievements.vs_win = true; checkAch();
  } else {
    vr.textContent = 'DERROTA - Tu: ' + G.score + ' vs IA: ' + G.vsOpponentScore;
    vr.style.color = '#f00'; playS('lose');
  }
}

// ============ WIFI MULTIPLAYER ============
function openMP() { show('mpPanel', 'block'); }
function closeMP() { hide('mpPanel'); }

// ============ DAILY CHALLENGE ============
function startDaily() {
  const today = new Date(); const seed = today.getFullYear() * 10000 + (today.getMonth() + 1) * 100 + today.getDate();
  const idx = seed % ALL_C.length;
  G.mode = 'challenge'; G.score = 0; G.combo = 0; G.maxCombo = 0; G.totalDrawPixels = 0; G.challengesCompleted = 0;
  G.powerups = { rainbow: 1, shield: 1, bomb: 1, mirror: 1, time: 1, freeze: 1 }; updatePW();
  drX.clearRect(0, 0, drC.width, drC.height); G.history = []; G.redoStack = [];
  G.challengeActive = true; G.challengeTimer = 60; G.challengesCompleted = 1;
  const ch = ALL_C[idx];
  document.getElementById('challengeText').textContent = 'DIARIO: ' + ch.name;
  document.getElementById('timerDisplay').textContent = '60s';
  hide('menuScreen'); hide('resultsScreen'); showOv('DESAFÍO DIARIO', ch.name); saveState(); playS('start');
}

// ============ POWERUPS ============
function updatePW() {
  document.getElementById('rainbowCount').textContent = G.powerups.rainbow;
  document.getElementById('shieldCount').textContent = G.powerups.shield;
  document.getElementById('bombCount').textContent = G.powerups.bomb;
  document.getElementById('mirrorCount').textContent = G.powerups.mirror;
  document.getElementById('timeCount').textContent = G.powerups.time;
  document.getElementById('freezeCount').textContent = G.powerups.freeze;
}

// ============ INPUT & INTERACTIONS ============
function isUI(e) {
  const t = e.target; if (!t) return false;
  if (t.closest('#hud') || t.closest('#toolbar') || t.closest('#pwBar') || t.closest('#cwPanel') || 
      t.closest('#mpPanel') || t.closest('#confirmDialog') || t.closest('#menuScreen') || t.closest('#resultsScreen')) return true;
  if (t.tagName === 'SELECT' || t.tagName === 'INPUT' || t.tagName === 'BUTTON') return true;
  return false;
}
function getPos(e) {
  const r = drC.getBoundingClientRect();
  const t = e.touches ? e.touches[0] : e;
  return { x: t.clientX - r.left, y: t.clientY - r.top };
}

function onDown(e) {
  if (isUI(e) || G.mode === 'menu') return;
  e.preventDefault();
  const p = getPos(e);
  G.drawing = true; G.lastX = p.x; G.lastY = p.y;
  saveState(); G.strokeCount++; G.totalStrokes++; playS('draw');
}
function onMove(e) {
  if (!G.drawing || G.mode === 'menu') return;
  e.preventDefault();
  const p = getPos(e);
  drawBrush(drX, p.x, p.y, G.lastX, G.lastY);
  if (G.mode === 'challenge' && G.challengeActive) addScore(1);
  if (G.mode === 'zen') addScore(1);
  G.lastX = p.x; G.lastY = p.y;
}
function onUp() { G.drawing = false; }

drC.addEventListener('mousedown', onDown);
drC.addEventListener('mousemove', onMove);
drC.addEventListener('mouseup', onUp);
drC.addEventListener('mouseleave', onUp);

drC.addEventListener('touchstart', onDown, { passive: false });
drC.addEventListener('touchmove', onMove, { passive: false });
drC.addEventListener('touchend', onUp);
drC.addEventListener('touchcancel', onUp);

// Clean event binding helper for menus and controls
function bindBtn(id, fn) {
  const b = document.getElementById(id);
  if (!b) return;
  b.addEventListener('click', (e) => {
    e.preventDefault();
    fn(e);
  });
}

// Top Bar
bindBtn('homeBtn', () => {
  if (G.mode === 'menu') return;
  showConfirm('¿Volver al menú?', 'Perderás tu progreso actual', goMenu);
});

function goMenu() {
  G.mode = 'menu'; G.vsMode = false; G.challengeActive = false;
  drX.clearRect(0, 0, drC.width, drC.height);
  document.getElementById('menuScreen').style.display = 'flex';
  document.getElementById('resultsScreen').style.display = 'none';
  document.getElementById('oppBox').style.display = 'none';
  document.getElementById('chatBox').style.display = 'none';
  document.getElementById('timerDisplay').textContent = '';
  document.getElementById('challengeText').textContent = 'Toca para dibujar';
}

// Toolbar
bindBtn('colorBtn', () => { document.getElementById('cwPanel').classList.toggle('show'); playS('color'); });
bindBtn('opacityBtn', () => {
  const v = [1, 0.8, 0.6, 0.4, 0.2];
  const i = v.indexOf(G.brushOpacity);
  G.brushOpacity = v[(i + 1) % v.length];
  document.getElementById('opacityBtn').textContent = G.brushOpacity.toFixed(1);
});
bindBtn('undoBtn', undo);
bindBtn('redoBtn', redo);
bindBtn('clearBtn', () => {
  showConfirm('¿Limpiar canvas?', 'Se borrará todo lo dibujado', () => { saveState(); drX.clearRect(0, 0, drC.width, drC.height); });
});
bindBtn('saveBtn', () => {
  const tc = document.createElement('canvas'); tc.width = drC.width; tc.height = drC.height;
  const t = tc.getContext('2d'); t.fillStyle = '#0a0a0f'; t.fillRect(0, 0, tc.width, tc.height);
  t.drawImage(drC, 0, 0);
  const l = document.createElement('a'); l.download = 'neon-canvas-' + Date.now() + '.png';
  l.href = tc.toDataURL(); l.click(); addScore(50);
});
bindBtn('gridBtn', () => { G.gridOn = !G.gridOn; document.getElementById('gridBtn').classList.toggle('active', G.gridOn); });
bindBtn('soundBtn', () => { G.soundOn = !G.soundOn; document.getElementById('soundBtn').classList.toggle('active', G.soundOn); if (G.soundOn) playS('color'); });

document.getElementById('modeSelect').addEventListener('change', e => G.brushMode = e.target.value);
document.getElementById('symSelect').addEventListener('change', e => G.symmetry = parseInt(e.target.value));

// Powerups
bindBtn('pwRainbow', () => { if (G.powerups.rainbow > 0) { G.powerups.rainbow--; G.activePowerups.rainbow = !G.activePowerups.rainbow; updatePW(); } });
bindBtn('pwShield', () => { if (G.powerups.shield > 0) { G.powerups.shield--; G.activePowerups.shield = true; setTimeout(() => G.activePowerups.shield = false, 5000); updatePW(); } });
bindBtn('pwBomb', () => {
  if (G.powerups.bomb > 0) {
    G.powerups.bomb--; saveState();
    const cx = drC.width / 2, cy = drC.height / 2;
    for (let i = 0; i < 80; i++) spawnP(cx, cy, ['#f00', '#ff0', '#0ff', '#f0f'][Math.floor(Math.random() * 4)], 1);
    drX.clearRect(0, 0, drC.width, drC.height); updatePW(); playS('boom');
  }
});
bindBtn('pwMirror', () => {
  if (G.powerups.mirror > 0) {
    G.mirrorOn = !G.mirrorOn;
    document.getElementById('pwMirror').style.borderColor = G.mirrorOn ? '#0ff' : 'rgba(255,255,255,0.25)';
  }
});
bindBtn('pwTime', () => { if (G.powerups.time > 0 && G.challengeActive) { G.powerups.time--; G.challengeTimer += 15; updatePW(); playS('combo'); } });
bindBtn('pwFreeze', () => {
  if (G.powerups.freeze > 0) {
    G.powerups.freeze--; G.activePowerups.freeze = true;
    document.getElementById('pwFreeze').style.borderColor = '#0ff';
    setTimeout(() => { G.activePowerups.freeze = false; document.getElementById('pwFreeze').style.borderColor = 'rgba(255,255,255,0.25)'; }, 5000);
    updatePW();
  }
});

// Menu Buttons Fix
bindBtn('btnFree', () => {
  G.mode = 'free'; G.score = 0; G.combo = 0; G.vsMode = false; G.mpMode = false;
  drX.clearRect(0, 0, drC.width, drC.height); G.history = []; G.redoStack = [];
  hide('menuScreen');
  document.getElementById('challengeText').textContent = 'Modo Libre';
  document.getElementById('timerDisplay').textContent = '';
  document.getElementById('oppBox').style.display = 'none';
  playS('start');
});
bindBtn('btnChallenge', startCh);
bindBtn('btnZen', () => {
  G.mode = 'zen'; G.score = 0; G.combo = 0; G.vsMode = false; G.mpMode = false;
  drX.clearRect(0, 0, drC.width, drC.height); G.history = []; G.redoStack = [];
  hide('menuScreen');
  document.getElementById('challengeText').textContent = 'Modo Zen';
  document.getElementById('timerDisplay').textContent = '';
  document.getElementById('oppBox').style.display = 'none';
  playS('start');
});
bindBtn('btnMulti', startVS);
bindBtn('btnWiFi', openMP);
bindBtn('btnDaily', startDaily);
bindBtn('btnAchieve', () => {
  const u = ACHS.filter(a => G.achievements[a.id]);
  showConfirm('LOGROS (' + u.length + '/' + ACHS.length + ')', u.map(a => a.name).join(', ') || 'Aún no has desbloqueado ningún logro.', () => {});
});
bindBtn('btnAgain', () => { if (G.vsMode) startVS(); else startCh(); });
bindBtn('btnMenu', () => goMenu());

bindBtn('mpClose', closeMP);
bindBtn('mpCreate', () => {
  document.getElementById('mpStatus').textContent = 'Modo multijugador activo localmente.';
  document.getElementById('mpStatus').style.color = '#0f0';
});
bindBtn('mpJoin', () => {
  document.getElementById('mpStatus').textContent = 'Conectando a sala...';
  document.getElementById('mpStatus').style.color = '#ff0';
});

// ============ COMBO DECAY & STATS ============
function comboTick() {
  if (G.comboTimer > 0) G.comboTimer--;
  else if (G.combo > 0) { G.combo = 0; document.getElementById('comboDisplay').style.opacity = '0'; }
}
function updateStats() {
  document.getElementById('statsBar').textContent = G.brushMode.toUpperCase() + ' | x' + G.symmetry + ' | ' + G.brushOpacity.toFixed(1) + ' op | ' + G.totalStrokes + ' trazos';
}

// ============ MAIN LOOP ============
function loop() {
  drawBg();
  drawP();
  if (G.gridOn) drawGrid();
  if (G.mode === 'challenge') chTick();
  if (G.vsMode) aiTick();
  comboTick();
  updateStats();
  checkAch();
  requestAnimationFrame(loop);
}

window.onload = function() {
  resize();
  loop();
};
