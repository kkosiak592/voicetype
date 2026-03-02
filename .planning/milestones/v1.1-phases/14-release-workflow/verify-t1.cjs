const fs = require('fs');
const doc = fs.readFileSync('RELEASING.md', 'utf8');
const checks = {
  hasVersionBump: /package\.json.*Cargo\.toml.*tauri\.conf\.json/s.test(doc),
  hasCommitCmd: /git commit/.test(doc),
  hasTagCmd: /git tag/.test(doc),
  hasPushCmd: /git push/.test(doc),
  hasChangelog: /CHANGELOG/.test(doc),
  hasSemver: /MAJOR.*MINOR.*PATCH/s.test(doc),
  hasTroubleshooting: /[Tt]roubleshooting/.test(doc)
};
console.log(checks);
if (!Object.values(checks).every(Boolean)) {
  console.error('FAIL');
  process.exit(1);
}
console.log('PASS');
