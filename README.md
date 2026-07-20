<p align="center">
  <img src="docs/img/banner.svg" alt="Organizer Banner" width="100%">
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <img alt="Status" src="https://img.shields.io/badge/status-pre--alpha-orange.svg">
  <img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-Rust-CE412B.svg">
  <img alt="Built with Tauri" src="https://img.shields.io/badge/built%20with-Tauri-24C8DB.svg">
  <a href="https://github.com/SulfusHQ/organizer/stargazers"><img alt="Stars" src="https://img.shields.io/github/stars/SulfusHQ/organizer?style=social"></a>
</p>

---

## About

**FileClassifier** (Organizer) is a lightning-fast, natively compiled desktop application designed to rescue your system from clutter. 

Downloads folders turn into graveyards: files with meaningless names (`document(3).pdf`, `IMG_20240512.png`), zero organization, and no easy way to know what's safe to delete. This application watches your folders in real-time, instantly categorizes new files, and automatically moves, renames, or schedules them for deletion based on a powerful custom rule engine.

Built on top of **Tauri** and **Rust**, it uses barely any system resources while delivering a stunning, macOS-inspired glassmorphism user interface.

## ✨ Core Features

* 🚀 **Real-Time Watcher:** Uses system-level hooks to instantly detect when new files are downloaded or created.
* 🧠 **Smart Rule Engine:** Chain together complex logic using `All`, `Any`, or `None` matching. Filter files by:
  * Extension (e.g., `.pdf`, `.png`)
  * File Name (contains, starts with, ends with)
  * File Size (greater than, less than)
* ⚡ **Automated Actions:**
  * **Move:** Automatically route files to specific destination folders (auto-creates folders if they don't exist).
  * **Rename:** Standardize your file names instantly.
  * **Scheduled Deletion:** Move a file to an archive and set it to automatically self-destruct after $X$ days.
* 🕰️ **The Background Reaper:** A lightweight background thread that periodically sweeps a hidden ledger to permanently delete files whose expiration timers have run out.
* 🎨 **Premium UI:** A fully custom, highly polished interface featuring dynamic blur effects, animations, and dark-mode aesthetics.

## 🛠️ Architecture & Tech Stack

This app relies on a split-architecture design for maximum performance and security:
- **Backend:** Written purely in **Rust**. Handles the file system watcher (via the `notify` crate), multithreaded scheduling, and local configuration persistence in your native AppData directory.
- **Frontend:** Written in standard **HTML/CSS/JS** for an ultra-lightweight client size (no heavy frameworks).
- **Framework:** **Tauri** is used as the bridge between the Rust backend and the web frontend, completely eliminating the need for bulky Chromium instances (like Electron).

## 🚀 Installation

Pre-compiled binaries are automatically generated for every release via GitHub Actions.

1. Navigate to the **[Releases](../../releases)** tab.
2. Download the appropriate installer for your system:
   * **macOS:** Download the `.dmg` file.
   * **Windows:** Download the `.exe` file.
3. Install and run!

*(Note for macOS users: If you receive an "App is damaged" warning, this is a standard macOS quarantine flag for unsigned apps. You can bypass this by running `xattr -cr /Applications/FileClassifier.app` in your terminal).*

## 💻 Development

If you want to compile the app from source or contribute to the project:

### Prerequisites
- [Node.js](https://nodejs.org/) (v20+)
- [Rust](https://rustup.rs/) (Stable)
- [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites)

### Running Locally
```bash
# Install frontend dependencies
npm install

# Run the app in development mode (with hot-reloading)
npm run tauri dev
```

### Building for Production
```bash
npm run tauri build
```
This will compile the optimized application into the `src-tauri/target/release/bundle/` directory.

## 📜 License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
