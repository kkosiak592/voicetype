# Options Comparison: Pushing Updates to Users of a Tauri 2 Desktop App

## Strategic Summary

Four viable options exist for delivering updates to users of your Tauri 2 voice-to-text app. **tauri-plugin-updater backed by GitHub Releases** is the clear winner: it provides seamless auto-update UX with zero hosting cost and minimal maintenance, automated end-to-end via GitHub Actions. The key tradeoff is a one-time medium-complexity setup (signing keys + CI workflow) versus zero-effort manual download links that give a worse user experience.

## Context

VoiceType is a Tauri 2 Windows desktop app (NSIS installer) currently in development. Distribution target is friends and colleagues (<20 users). No update infrastructure exists yet. The goal is seamless auto-updates so users don't have to manually re-download installers.

**Important:** This is a Tauri 2 app, not Electron. Tauri has its own updater ecosystem (`tauri-plugin-updater`) that is different from Electron's Squirrel/electron-updater.

## Decision Criteria

1. **User Experience** - How seamless is the update for end users? - Weight: **High**
2. **Setup Complexity** - One-time effort to get it working for a solo dev - Weight: **High**
3. **Ongoing Maintenance** - How much work per release after initial setup? - Weight: **Medium**
4. **Cost** - Monthly/yearly expenses - Weight: **Medium**
5. **Reliability** - Can users always get updates? Edge cases? - Weight: **Medium**
6. **Infrastructure** - Servers, services, accounts needed - Weight: **Low**

## Options

### Option A: tauri-plugin-updater + GitHub Releases

Tauri's official updater plugin configured to pull updates from GitHub Releases. `tauri-action` in GitHub Actions automatically builds, signs, generates `latest.json`, and uploads everything to a Release. The app checks `https://github.com/OWNER/REPO/releases/latest/download/latest.json` on launch.

- **User Experience**: Excellent - fully automatic download, verify, install, relaunch. User sees a prompt, clicks "Update", app restarts with new version.
- **Setup Complexity**: Medium - generate Ed25519 signing keypair, configure `tauri.conf.json`, create GitHub Actions workflow with `tauri-action`. One-time ~2-4 hours.
- **Ongoing Maintenance**: Minimal - bump version in `tauri.conf.json`, push a git tag, CI does everything else.
- **Cost**: Free (public repo) or included in GitHub plan (private repo Actions minutes).
- **Reliability**: High - GitHub CDN serves release assets globally. Signing ensures integrity.
- **Infrastructure**: GitHub only.
- **Score: 9/10**

### Option B: GitHub Releases + Manual Download Notification

App checks GitHub API for latest release version. If newer, shows a notification with a link. User opens browser, downloads installer manually, runs it.

- **User Experience**: Poor - user must close app, download file, run installer manually. Many will ignore it.
- **Setup Complexity**: Low - single API call + a dialog. ~1 hour of work.
- **Ongoing Maintenance**: Minimal - upload installer to GitHub Release manually or via CI.
- **Cost**: Free.
- **Reliability**: Medium - depends on user actually following through. GitHub API rate limit (60 req/hr unauthenticated).
- **Infrastructure**: GitHub only.
- **Score: 5/10**

### Option C: tauri-plugin-updater + CrabNebula Cloud

Same updater plugin but backed by CrabNebula's managed CDN and distribution platform (official Tauri partner).

- **User Experience**: Excellent - identical to Option A from user perspective.
- **Setup Complexity**: Low-Medium - CrabNebula has documented integration path, handles hosting.
- **Ongoing Maintenance**: Minimal - similar to Option A, CrabNebula manages infrastructure.
- **Cost**: ~EUR 9/month base + overage fees. 14-day free trial. Open source discount available.
- **Reliability**: High - purpose-built CDN for Tauri apps, analytics included.
- **Infrastructure**: CrabNebula account + GitHub for CI.
- **Score: 7/10**

### Option D: tauri-plugin-updater + Custom Self-Hosted Server

Same updater plugin but backed by your own server that serves update manifests and binaries.

- **User Experience**: Excellent - identical to Option A from user perspective.
- **Setup Complexity**: High - build and deploy a server (FastAPI/Express/Cloudflare Worker), configure storage, TLS.
- **Ongoing Maintenance**: Moderate - server uptime, storage management, deployment pipeline.
- **Cost**: $0-20/month (VPS or serverless).
- **Reliability**: Depends on your infrastructure. Single point of failure if self-hosted.
- **Infrastructure**: Web server + file storage + TLS certificate.
- **Score: 4/10**

## Comparison Matrix

| Criterion (Weight)            | A: Plugin + GitHub | B: Manual Download | C: Plugin + CrabNebula | D: Plugin + Custom Server |
|-------------------------------|--------------------|--------------------|-------------------------|---------------------------|
| User Experience (High)        | Excellent          | Poor               | Excellent               | Excellent                 |
| Setup Complexity (High)       | Medium             | Low                | Low-Medium              | High                      |
| Ongoing Maintenance (Med)     | Minimal            | Minimal            | Minimal                 | Moderate                  |
| Cost (Med)                    | Free               | Free               | ~EUR 9/mo               | $0-20/mo                  |
| Reliability (Med)             | High               | Medium             | High                    | Variable                  |
| Infrastructure (Low)          | GitHub             | GitHub             | CrabNebula + GitHub     | Server + Storage          |
| **Score**                     | **9/10**           | **5/10**           | **7/10**                | **4/10**                  |

## Recommendation

**Option A: tauri-plugin-updater + GitHub Releases** because it delivers the best UX at zero cost with minimal maintenance. For a solo dev distributing to <20 friends/colleagues, paying for CrabNebula or running a custom server adds complexity and cost with no meaningful benefit. GitHub Releases as the update backend is explicitly supported and recommended by Tauri maintainers.

## Runner-up

**Option C: CrabNebula Cloud** - choose this if you later need download analytics, global CDN performance, or grow to hundreds of users and want managed infrastructure. Switching from GitHub Releases to CrabNebula is straightforward (change the endpoint URL in `tauri.conf.json`).

## Implementation Guide for Option A

### Prerequisites

- GitHub repo (public or private)
- GitHub Actions enabled

### Step 1: Generate Signing Keys

```bash
npm run tauri signer generate -- -w ~/.tauri/voicetype.key
```

This creates:
- `~/.tauri/voicetype.key` (private key - KEEP SECRET)
- `~/.tauri/voicetype.key.pub` (public key - embed in app)

**CRITICAL:** Back up the private key securely. If lost, all existing installs can never receive updates.

### Step 2: Configure tauri.conf.json

Add to your existing config:

```json
{
  "bundle": {
    "createUpdaterArtifacts": true
  },
  "plugins": {
    "updater": {
      "pubkey": "<CONTENTS OF voicetype.key.pub>",
      "endpoints": [
        "https://github.com/OWNER/REPO/releases/latest/download/latest.json"
      ]
    }
  }
}
```

### Step 3: Add Dependencies

```bash
# Rust plugin
cargo add tauri-plugin-updater --manifest-path src-tauri/Cargo.toml
cargo add tauri-plugin-process --manifest-path src-tauri/Cargo.toml

# JS bindings
npm install @tauri-apps/plugin-updater @tauri-apps/plugin-process
```

Register plugins in `src-tauri/src/lib.rs`:
```rust
tauri::Builder::default()
    .plugin(tauri_plugin_updater::Builder::new().build())
    .plugin(tauri_plugin_process::init())
    // ... existing plugins
```

### Step 4: Add Update Check to Frontend

```typescript
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

async function checkForUpdate() {
  const update = await check();
  if (update) {
    // update.version, update.body (release notes) available
    await update.downloadAndInstall((event) => {
      // Optional: track download progress
      if (event.event === 'Progress') {
        console.log(`Downloaded ${event.data.chunkLength} bytes`);
      }
    });
    await relaunch();
  }
}
```

### Step 5: Add Capabilities Permission

In your capabilities JSON file:
```json
{
  "permissions": [
    "updater:default",
    "process:allow-restart"
  ]
}
```

### Step 6: GitHub Actions Workflow

```yaml
name: Release
on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - uses: dtolnay/rust-toolchain@stable
      - run: npm install

      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: v__VERSION__
          releaseName: "VoiceType v__VERSION__"
          releaseBody: "See the changelog for details."
          releaseDraft: false
          prerelease: false
          uploadUpdaterJson: true
          updaterJsonPreferNsis: true
```

### Step 7: Store Secrets in GitHub

In your repo Settings > Secrets and variables > Actions:
- `TAURI_SIGNING_PRIVATE_KEY`: contents of `~/.tauri/voicetype.key`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: password you chose during key generation

### Release Workflow (ongoing)

1. Bump version in `src-tauri/tauri.conf.json`
2. Commit: `git commit -m "release: v1.0.1"`
3. Tag: `git tag v1.0.1`
4. Push: `git push && git push --tags`
5. GitHub Actions builds, signs, creates Release with `latest.json`
6. Users' apps automatically detect and install the update

## Important Caveats

| Issue | Detail |
|-------|--------|
| **Windows SmartScreen** | Without a Windows code signing certificate (~$200-400/yr), users see "Windows protected your PC" on first install. This is separate from Tauri's Ed25519 signing. Updates after first install won't trigger it again. |
| **App exits during update** | On Windows, the NSIS installer requires the app to close. The updater handles this automatically but users should be warned. |
| **Private repos** | If your repo is private, add authentication headers to the updater config or make releases public. |
| **Key backup** | The Ed25519 private key is irreplaceable. Store it in a password manager AND as a GitHub secret. |
| **No delta updates** | Every update downloads the full installer (~50-100MB+ with CUDA models bundled). For your app with large ML models, consider whether models should be downloaded separately. |

## Sources

- [Tauri v2 Updater Plugin Documentation](https://v2.tauri.app/plugin/updater/)
- [tauri-apps/tauri-action GitHub](https://github.com/tauri-apps/tauri-action)
- [Tauri GitHub Discussion #10206](https://github.com/orgs/tauri-apps/discussions/10206) - Maintainer confirmation of GitHub Releases approach
- [CrabNebula Cloud Pricing](https://crabnebula.dev/cloud/pricing/)
- [Tauri Windows Code Signing](https://v2.tauri.app/distribute/sign/windows/)
- [faynoSync - Self-hosted update server](https://github.com/ku9nov/faynoSync)
