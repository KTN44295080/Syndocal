// Regenerate the media-free acceptance fixture; no machine paths are embedded.
import { readFileSync, writeFileSync } from 'node:fs';
const template = JSON.parse(readFileSync(new URL('../../../samples/phase1-mini-show.sdc', import.meta.url), 'utf8'));
const font = {
  ' ': ['000','000','000','000','000'], '-': ['000','000','111','000','000'],
  '(': ['010','100','100','100','010'], ')': ['010','001','001','001','010'],
  A: ['010','101','111','101','101'], B: ['110','101','110','101','110'],
  D: ['110','101','101','101','110'], E: ['111','100','110','100','111'],
  F: ['111','100','110','100','100'], G: ['111','100','101','101','111'],
  H: ['101','101','111','101','101'], I: ['111','010','010','010','111'],
  L: ['100','100','100','100','111'], M: ['101','111','111','101','101'],
  N: ['101','111','111','111','101'], O: ['111','101','101','101','111'],
  P: ['110','101','110','100','100'], R: ['110','101','110','101','101'],
  T: ['111','010','010','010','010'], U: ['101','101','101','101','111'],
};
const glyph = c => font[c].flatMap(row => [...row]).reduce((n, bit, i) => n + Number(bit) * 2 ** i, 0);
const label = (s, x, y) => [...s].map((c, i) => `  ink = max(ink, letter(p - vec2(${x + i * 24}.0, ${y}.0), ${glyph(c)}.0));`).join('\n');
const shader = `/*{"ISFVSN":"2","DESCRIPTION":"Syndocal Syphon orientation, channel and motion acceptance","INPUTS":[]}*/
float letter(vec2 local, float bits) {
  vec2 cell = floor(local / 6.0);
  if (cell.x < 0.0 || cell.x >= 3.0 || cell.y < 0.0 || cell.y >= 5.0) return 0.0;
  return mod(floor(bits / pow(2.0, cell.y * 3.0 + cell.x)), 2.0);
}
void main() {
  vec2 p = isf_FragNormCoord * vec2(1920.0, 1080.0);
  vec3 c = vec3(0.08 + 0.04 * mod(floor(p.x / 80.0) + floor(p.y / 80.0), 2.0));
  vec2 center = vec2(960.0 + 300.0 * sin(TIME), 540.0 + 150.0 * cos(TIME));
  if (distance(p, center) < 80.0) c = vec3(1.0, 0.8, 0.0);
  if (p.x < 620.0 && p.y < 150.0) c = vec3(1.0, 0.0, 0.0);
  if (p.x >= 1300.0 && p.y < 150.0) c = vec3(0.0, 1.0, 0.0);
  if (p.x < 620.0 && p.y >= 930.0) c = vec3(0.0, 0.0, 1.0);
  float ink = 0.0;
${label('TOP-LEFT (RED)', 24, 24)}
${label('TOP-RIGHT (GREEN)', 1324, 24)}
${label('BOTTOM-LEFT (BLUE)', 24, 954)}
  gl_FragColor = vec4(mix(c, vec3(1.0), ink), 1.0);
}
`;
const snapshot = template.snapshot;
snapshot.fixtures = []; snapshot.cues = []; snapshot.submasters = [];
snapshot.timeline = { id: 1, label: 'Syphon acceptance', events: [], automations: [], video_automations: [], audio: null, playing: false, position_ms: 0, duration_ms: 60000 };
snapshot.output.enabled = false; snapshot.dmx_outputs = [];
snapshot.stage_objects = []; snapshot.stage_map_presets = [];
const layer = snapshot.video.layers[0];
layer.label = 'Syphon RGB corners + moving gold circle';
// Static generators deliberately retain PTS zero. An animated source supplies
// the media clock used by ISF TIME; the shader replaces all of its pixels.
layer.source = { kind: { Generator: { Noise: { seed: 92, scale: 5, speed: 100, color_a: [0, 0, 0, 255], color_b: [255, 255, 255, 255] } } }, path: null, name: 'Media-free animated acceptance', codec: null, metadata: null };
layer.state.opacity = 1; layer.state.playing = true;
layer.state.bpm_sync.enabled = false; layer.state.cue_points = []; layer.state.cue_points_ms = [];
layer.state.loop_end_ms = 60000;
layer.isf_effect = { enabled: true, label: 'Syphon RGB orientation + motion', source: shader, controls: [], categories: ['Acceptance'] };
snapshot.video.compositions = [{ id: 1, label: 'Syphon 1920x1080', layer_ids: [1], output_ids: [1] }];
snapshot.video.outputs = [{ id: 1, label: 'Syndocal-Syphon-Test', kind: 'SyphonServer', enabled: true, composition_id: 1, fullscreen: false, monitor_id: null, width: 1920, height: 1080, endpoint_name: 'Syndocal-Syphon-Test', opacity: 1, blackout: false }];
snapshot.video.mapping_presets = [];
const project = { version: 2, app: 'Syndocal', custom_profiles: [], fixture_groups: [], snapshot };
writeFileSync(new URL('./syphon-acceptance-project.sdc', import.meta.url), JSON.stringify(project, null, 2) + '\n');
