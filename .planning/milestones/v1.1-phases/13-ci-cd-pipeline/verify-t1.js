const yaml = require('fs').readFileSync('.github/workflows/release.yml', 'utf8');
const lines = yaml.split('\n');
const hasTrigger = lines.some(l => l.includes('v*'));
const hasTauri = lines.some(l => l.includes('tauri-apps/tauri-action'));
const hasSigning = lines.some(l => l.includes('TAURI_SIGNING_PRIVATE_KEY'));
const hasCuda = lines.some(l => /cuda/i.test(l));
const hasLlvm = lines.some(l => /llvm/i.test(l));
const hasUpdaterJson = lines.some(l => l.includes('includeUpdaterJson'));
console.log({ hasTrigger, hasTauri, hasSigning, hasCuda, hasLlvm, hasUpdaterJson });
const pass = hasTrigger && hasTauri && hasSigning && hasCuda && hasLlvm && hasUpdaterJson;
if (!pass) { console.error('FAIL: Missing required elements'); process.exit(1); }
console.log('PASS: All required elements present');
