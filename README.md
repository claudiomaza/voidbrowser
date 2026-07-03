# VoidBrowser 🧩🛡️

> Fork multiplataforma de [glebschkv/voidbrowser](https://github.com/glebschkv/voidbrowser) — adaptado por cm2labs.

Un navegador de privacidad "zero-tracking" basado en **Tauri v2**, ahora con build para **Windows, Linux, macOS, Android e iOS**.

## ✨ Features originales
- **Ad blocking** con EasyList + EasyPrivacy (Brave adblock engine)
- **Fingerprint resistance** — spoofing de canvas, WebGL, AudioContext
- **HTTPS-only mode** forzado
- **Bookmarks cifrados** con SQLCipher (ChaCha20-Poly1305 + Argon2)
- **Navegación efímera** — cookies y cache en RAM, se destruyen al salir
- **WebRTC leak prevention**
- **Keyring persistente** — la sesión sobrevive a reinicios

## 🎮 Configuración de Build

En `.github/workflows/build.yml`, buscá esta sección y poné `'true'`/`'false'`:

```yaml
env:
  WINDOWS: "true"
  LINUX: "false"
  MACOS: "false"
  ANDROID: "false"
  IOS: "false"
```

| Plataforma | Binario | Runner CI |
|---|---|---|
| 🪟 Windows | `.exe` + NSIS installer | `windows-latest` |
| 🐧 Linux | `.deb` + AppImage | `ubuntu-latest` |
| 🍏 macOS | `.dmg` | `macos-latest` |
| 🤖 Android | `.apk` | `ubuntu-latest` (+ SDK) |
| 📱 iOS | `.app` (simulador) | `macos-latest` |

## 🚀 Stack
| Capa | Tecnología |
|---|---|
| Frontend | SolidJS + TypeScript + Tailwind CSS 4 |
| Build | Vite 6 |
| Backend | Rust + Tauri v2 |
| Ad blocking | `adblock` crate (EasyList + EasyPrivacy) |
| Storage | SQLCipher (ChaCha20-Poly1305 + Argon2) |
| Keyring | `keyring` crate (OS native: Credential Manager / Keychain / Secret Service) |

## 📁 Estructura
```
.
├── .github/workflows/build.yml   ← CI/CD con toggles
├── src/                          ← Frontend SolidJS
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── browser/              ← Tabs, navegación, webview
│       ├── privacy/              ← Ad blocker, fingerprint, HTTPS-only
│       ├── storage/              ← SQLCipher, bookmarks, history, settings
│       └── commands.rs           ← Tauri IPC commands
├── package.json
└── pnpm-lock.yaml
```

## ⚡ Built with cm2labs

