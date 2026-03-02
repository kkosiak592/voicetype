const fs = require('fs');
const doc = fs.readFileSync('CHANGELOG.md', 'utf8');
const checks = {
  hasUnreleased: /## \[Unreleased\]/.test(doc),
  hasVersion: /## \[0\.1\.0\]/.test(doc),
  hasAdded: /### Added/.test(doc),
  hasKeepAChangelog: /keepachangelog/.test(doc),
  hasSemver: /semver\.org/.test(doc),
  hasCompareLink: /compare\//.test(doc),
  hasReleaseLink: /releases\/tag\//.test(doc)
};
console.log(checks);
if (!Object.values(checks).every(Boolean)) {
  console.error('FAIL');
  process.exit(1);
}
console.log('PASS');
