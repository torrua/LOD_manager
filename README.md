# LOD Manager

A desktop + mobile editor for the [Loglan](http://www.loglan.org) Online Dictionary (LOD).

Built with [Tauri v2](https://tauri.app) · Svelte 5 · Rust · SQLite (FTS5)

---

## Downloads

Pre-built binaries are on the [Releases](../../releases) page:

| Platform          | File                        |
| ----------------- | --------------------------- |
| Windows installer | `lod-manager_x64-setup.exe` |
| Windows MSI       | `lod-manager_x64.msi`       |
| Android (debug)   | `lod-manager-debug.apk`     |

---

## Development

### Prerequisites

- [Node.js](https://nodejs.org) 20 LTS
- [Rust](https://rustup.rs) stable (msvc toolchain on Windows)
- **Windows only:** [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/downloads/) with "Desktop development with C++"
- WebView2 (ships with Windows 11; installer available from Microsoft)

### Run in development

```bash
npm install                         # install JS deps + linters
rustup component add rustfmt clippy # install Rust tools (once)
npm run tauri                       # starts Vite + Tauri dev window
```

### Build for Windows

```bash
npm run tauri build
# Output: src-tauri/target/release/bundle/msi/
#         src-tauri/target/release/bundle/nsis/
```

### Build for Android (local)

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi \
                   x86_64-linux-android i686-linux-android
cargo install cargo-ndk

export JAVA_HOME=/path/to/jdk17
export ANDROID_HOME=/path/to/android-sdk
export NDK_HOME=$ANDROID_HOME/ndk/26.3.11579264

npm run tauri android init   # once — generates src-tauri/gen/android/
npm run tauri android build --debug
```

---

## Releasing a new version

1. Bump version in **two files**:
   - `package.json` → `"version"`
   - `src-tauri/tauri.conf.json` → `"version"`

2. Commit and push a tag:

```bash
git add package.json src-tauri/tauri.conf.json
git commit -m "chore: bump version to 1.2.3"
git tag v1.2.3
git push && git push --tags
```

GitHub Actions builds Windows MSI + debug APK and attaches them to a new GitHub Release automatically.

---

## Signed Android APK (optional)

For Google Play or direct signed distribution:

1. Generate a keystore:

```bash
keytool -genkey -v -keystore release.jks \
        -alias lod-manager -keyalg RSA -keysize 2048 -validity 10000
```

2. Add **repository secrets** (Settings → Secrets → Actions):

| Secret                      | Value                    |
| --------------------------- | ------------------------ |
| `ANDROID_KEYSTORE_BASE64`   | `base64 -w0 release.jks` |
| `ANDROID_KEYSTORE_PASSWORD` | keystore password        |
| `ANDROID_KEY_ALIAS`         | `lod-manager`            |
| `ANDROID_KEY_PASSWORD`      | key password             |

3. Run **"Android Signed Release"** workflow manually from the Actions tab.

---

## Importing data

The app reads original LOD text files (@ delimited):

| File                 | Contents                                     |
| -------------------- | -------------------------------------------- |
| `Words.txt`          | Word metadata (type, rank, match %, origins) |
| `WordSpell.txt`      | Spellings and event references               |
| `WordDefinition.txt` | Definitions                                  |
| `LexEvent.txt`       | Lexical events                               |
| `Author.txt`         | Authors                                      |
| `Type.txt`           | Word types                                   |

Use **Tools → Import** and select all files at once.
