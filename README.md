# LOD Manager

A desktop + mobile editor for the [Loglan](http://www.loglan.org) Online Dictionary (LOD).

Built with [Tauri v2](https://tauri.app) · Svelte 5 · Rust · SQLite (FTS5)

---

## Downloads

Pre-built binaries are on the [Releases](../../releases) page:

| Platform          | File                              |
| ----------------- | --------------------------------- |
| Windows installer | `LOD.Manager_1.6.x_x64-setup.exe` |
| Windows MSI       | `LOD.Manager_1.6.x_x64_en-US.msi` |
| Android (ARM64)   | `LOD.Manager_1.6.x_aarch64.apk`   |

---

## Development

### Prerequisites

- [Node.js](https://nodejs.org) 20 LTS
- [Rust](https://rustup.rs) stable (msvc toolchain on Windows)
- **Windows only:** [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/downloads/) with "Desktop development with C++"
- WebView2 (ships with Windows 11; installer available from Microsoft)

### Available commands

| Command                | Description                          |
| ---------------------- | ------------------------------------ |
| `npm run dev`          | Start Vite dev server                |
| `npm run tauri`        | Run Tauri app (dev mode)             |
| `npm run build`        | Build production bundle              |
| `npm run check`        | TypeScript check                     |
| `npm run lint`         | ESLint                               |
| `npm run format:check` | Prettier check only                  |
| `npm run ci:check`     | All frontend checks (format+lint+ts) |
| `npm run rust:fmt`     | Rustfmt format                       |
| `npm run rust:lint`    | Clippy lint (denies warnings)        |

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
     (current: `1.6.9`)

2. Commit and push a tag:

```bash
git add package.json src-tauri/tauri.conf.json
git commit -m "chore: bump version to 1.7.0"
git tag v1.7.0
git push && git push --tags
```

GitHub Actions builds Windows MSI + signed APK and attaches them to a new GitHub Release automatically.

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
