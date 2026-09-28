# KanzenCheki

[English](README.md) · [简体中文](README.zh-CN.md)

A local-first desktop app for organizing cheki collections, their images, and the people and memories behind them. Built with Tauri 2, Rust, SQLite, SvelteKit, and Svelte 5.

The name is a playful nod to the *kanzenseiiki* call.

> **Project status:** Version 0.1.0 is the first version and is currently in review. Features and data handling are being checked before a general release. Tagged pre-release packages are published through GitHub Actions.

## Features

- **Collection-first library:** Keep multiple scans or photos of one physical cheki together, with dates, people, groups, events, tags, notes, and favorites.
- **Local storage:** Store metadata in SQLite and originals as ordinary files. Import into the managed library or manage originals in configured external folders. Cached previews allow browsing when an external folder is offline.
- **Organize and find:** Search text, people, and tags; filter favorites and items needing details; sort collections; batch edit and merge.
- **Image tools:** Choose a cover, crop, correct perspective, rotate, and inspect the original at full size. Crops preserve the original; rotation updates the original file.
- **People and backups:** Keep person profiles, attachments, and Markdown notes. Create full or incremental library backups and preview conflicts before restoring.
- **Offline suggestions:** On-device face detection and matching can suggest shot types and people for review. Suggestions are never silently treated as verified metadata.
- **Interface languages:** Simplified Chinese, English, and Japanese.

The desktop app persists your library. The browser preview uses temporary sample data and resets on refresh. The statistics route exists but its page is not implemented yet.

> [!CAUTION]
> This is a Vibe Coding First project, and the code has not yet received a complete human review. Data loss or corruption is possible. Use the built-in backup tools regularly, and keep manual backups to reduce the risk.
>
> The project is still in pre-release and may introduce breaking changes.

## Getting started

### Requirements

[Devbox](https://www.jetify.com/devbox) provides Node.js 22, Rust, and the Linux GTK/WebKit dependencies used by this repository. Native desktop builds have been exercised on Linux; other platforms still need validation.

~~~sh
devbox run npm ci
devbox run desktop
~~~

The desktop development window starts a Vite server on port 1420. To inspect the temporary browser demo instead, run:

~~~sh
devbox run npm run demo:images          # Download third-party demo images locally
devbox run dev
~~~

### Desktop packages

Version tags such as `v0.1.0` trigger GitHub Actions to publish a [GitHub pre-release](https://github.com/V2lsbGlhbVpoYW5n/kanzencheki/releases) after all packages build successfully. The release contains a Windows x64 NSIS installer, macOS DMGs for Intel and Apple Silicon, and a Linux x64 AppImage. Pull requests and pushes to `main` run the frontend and Rust checks.

The macOS packages use ad hoc signing without notarization, so macOS may require manual approval before opening them. The Windows installer is unsigned and may trigger a SmartScreen warning. Do not download release files from third-party mirrors.

ImageMagick 7 is required for some image operations, and FFmpeg is required for video cover extraction. These tools are not included in the desktop packages.

### Build and verify

~~~sh
devbox run check                         # Svelte checks and frontend build
devbox run npm test                      # Frontend tests
devbox run cargo test --manifest-path src-tauri/Cargo.toml --lib
devbox run npm run desktop:build         # Desktop executable, without an installer
devbox run desktop-release               # Run the built executable
~~~

On Linux, the executable is at .cache/cargo-target/release/kanzencheki. The build does not currently produce an installer.

## Library and data

By default, the desktop app opens a library in its operating system application-data directory. On Linux this is typically:

~~~text
~/.local/share/app.kanzencheki.desktop/library/
~~~

The historical application identifier is deliberately retained so existing libraries remain discoverable after the rename. Set CHEKI_LIBRARY_DIR before launching to use another library directory. This environment variable is also retained for compatibility.

A library contains a SQLite database, managed originals, previews, and import staging files. Originals imported into the managed library are copied without changing their source files. Originals in configured external folders can be renamed or rotated in place by the app. Back up both the database and the originals; use the built-in backup tools for a consistent snapshot rather than copying only the live SQLite file.

Supported image imports include JPEG, PNG, WebP, and TIFF. Some preview operations use ImageMagick or a Rust decoder fallback. The desktop app starts with an empty library; sample images are only used by the browser demo.

## Project layout

| Path | Purpose |
| --- | --- |
| src/ | SvelteKit interface, localizations, and browser demo |
| src-tauri/ | Rust desktop backend, storage, and image processing |
| src-tauri/models/ | On-device model details and third-party licenses |
| static/demo/ | Browser demo image sources; images are downloaded locally on request |
| docs/ | Deprecated design artifacts retained for history |

## Roadmap

- [x] CI/CD for checks, builds, and releases
- [ ] Mobile app
- [ ] Local network synchronization
- [ ] Statistics page

These are directions for future work, not release commitments.

## Contributing

Bug reports and feature suggestions are welcome in GitHub Issues. Please describe the behavior, steps to reproduce, and your environment for bugs. This app is primarily built around a personal workflow. It will be maintained long term, but new features may be added selectively rather than aggressively.

## License

The project source code is licensed under the [MIT License](LICENSE). Bundled face models have their own licenses in [src-tauri/models/](src-tauri/models/README.md). Browser demo images are third-party material and are not included in the Git repository; see [their source notes](static/demo/SOURCES.md). The MIT license does not grant rights to those images or models. The download script checks each image's SHA-256 and stops if its source has changed.
