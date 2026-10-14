# 🎬 VidNest - Lokale KI-Video-Intelligenz-Zentrale & Medien-Hub

<p align="center">
  <img src="../../src-tauri/icons/icon.png" width="128" height="128" alt="VidNest Logo" />
</p>

<p align="center">
  <strong>Open-Source-Desktop-Workspace der nächsten Generation für Video-Intelligenz, Inhaltsverständnis & Asset-Management</strong><br>
  100% Privat & Local-First · KI-Zusammenfassungen & Themen-Zeitachsen · Millisekunden-FTS5-Volltextsuche · Eigenständiges Bild-in-Bild · Plattformübergreifend Native
</p>

<p align="center">
  <a href="https://github.com/NovaMindLab/VidNest/releases"><img src="https://img.shields.io/github/v/release/NovaMindLab/VidNest?style=flat-square&color=blue" alt="Release"></a>
  <a href="https://github.com/NovaMindLab/VidNest/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg?style=flat-square" alt="License"></a>
  <a href="https://tauri.app/"><img src="https://img.shields.io/badge/Tauri-2.0-24C8D8?style=flat-square&logo=tauri&logoColor=white" alt="Tauri"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-2021-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Rust"></a>
  <a href="https://vuejs.org/"><img src="https://img.shields.io/badge/Vue-3.5-4FC08D?style=flat-square&logo=vue.js&logoColor=white" alt="Vue"></a>
</p>

<div align="center">
  <details>
    <summary>🌐 <b>Sprachauswahl / Language / 言語 / 언어 / Sprache / Idioma (6 Languages)</b></summary>
    <br/>
    <p>
      <a href="../../README.md">🇺🇸 English</a> &nbsp;|&nbsp; 
      <a href="../../README.zh-CN.md">🇨🇳 简体中文</a> &nbsp;|&nbsp; 
      <a href="README.ja.md">🇯🇵 日本語</a> &nbsp;|&nbsp; 
      <a href="README.ko.md">🇰🇷 한국어</a> &nbsp;|&nbsp; 
      <a href="README.de.md">🇩🇪 <b>Deutsch</b></a> &nbsp;|&nbsp; 
      <a href="README.es.md">🇪🇸 Español</a>
    </p>
  </details>
</div>

<p align="center">
  <a href="https://novamindlab.github.io/VidNest/">🌐 Offizielles Portal & Download-Center</a> •
  <a href="../../wiki/README.md">📚 Entwickler-Dokumentation</a> •
  <a href="https://github.com/NovaMindLab/VidNest/releases/latest">⚡ Neueste Version herunterladen</a>
</p>

---

## 🌟 Warum VidNest?

**VidNest** ist eine hochperformante, ressourcenschonende **Local-First KI-Desktop-Anwendung**, entwickelt für Videoproduzenten, Forscher und Medienliebhaber.

### 🚀 Kernfunktionen

* 🧠 **KI-Video-Intelligenz-Zentrale**: Kombiniert konfigurierungsfreie lokale NLP-Algorithmen mit modernen LLMs (OpenAI, DeepSeek, Ollama) und Whisper-Spracherkennung. Erstellt Inhaltszusammenfassungen, Themen-Zeitachsen und intelligente Tags vollautomatisch.
* ⚡ **Millisekunden-FTS5-Volltextsuche**: Integrierte SQLite-FTS5-Engine zur blitzschnellen Suche nach Titeln, Tags und gesprochenen Dialogen mit direktem Zeitsprung.
* 🔍 **Mehrdimensionale Video-Deduplizierung**: Intelligente Erkennung doppelter Videos über SHA-256 Block-Hashes, Auflösung, Bitrate und Dateinamensmuster. Gibt Speicherplatz sicher frei.
* 🎵 **Hi-Fi Audio-Extraktion**: Unterstützt MP3, M4A, WAV und FLAC. Verlustfreie Stream-Kopie (Direct Stream Copy) ohne Neucodierung und Qualitätsverlust in Millisekunden.
* 🌐 **Integriertes yt-dlp Download-Center**: Videos und Audios von über 1.000 Plattformen (YouTube, TikTok, X, Instagram etc.) unkompliziert lokal sichern.

---

## 🛠️ Technologie-Stack

* **Kern-Framework**: Tauri v2, Rust 2021
* **Datenbank**: SQLite3 (mit aktivierter FTS5-Volltexterweiterung)
* **Frontend-UI**: Vue 3.5, TypeScript, Vite
* **Medienverarbeitung**: FFmpeg, FFprobe, Whisper, yt-dlp

---

## 📄 Lizenz

Dieses Projekt ist unter der [MIT-Lizenz](../../LICENSE) als Open Source lizenziert.
