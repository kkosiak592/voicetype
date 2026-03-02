const fs = require('fs');
const y = fs.readFileSync('.github/workflows/release.yml', 'utf8');
try {
  const lines = y.split('\n');
  let checks = {
    draft: false,
    updater: false,
    nsisPrefer: false,
    signing: false,
    token: false
  };
  for (const l of lines) {
    if (l.includes('releaseDraft: false')) checks.draft = true;
    if (l.includes('includeUpdaterJson: true')) checks.updater = true;
    if (l.includes('updaterJsonPreferNsis: true')) checks.nsisPrefer = true;
    if (l.includes('TAURI_SIGNING_PRIVATE_KEY:') && l.includes('secrets.')) checks.signing = true;
    if (l.includes('GITHUB_TOKEN:') && l.includes('secrets.')) checks.token = true;
  }
  const pass = Object.values(checks).every(Boolean);
  console.log(checks);
  if (!pass) { console.error('FAIL: Missing config'); process.exit(1); }
  console.log('PASS: Workflow structure validated');
} catch (e) {
  console.error(e.message);
  process.exit(1);
}
