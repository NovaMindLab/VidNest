# 🎬 VidNest - Centro de Inteligencia de Video Local por IA y Gestión Multimedia

<p align="center">
  <img src="../../src-tauri/icons/icon.png" width="128" height="128" alt="VidNest Logo" />
</p>

<p align="center">
  <strong>Espacio de Trabajo de Escritorio de Próxima Generación para Inteligencia de Video, Comprensión de Contenido y Gestión de Activos</strong><br>
  100% Privado y Local-First · Resúmenes Ejecutivos por IA y Cronología de Temas · Búsqueda FTS5 en Milisegundos · Reproductor Flotante Independiente · Nativo Multiplataforma
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
    <summary>🌐 <b>Seleccionar Idioma / Language / 言語 / 언어 / Sprache / Idioma (6 Languages)</b></summary>
    <br/>
    <p>
      <a href="../../README.md">🇺🇸 English</a> &nbsp;|&nbsp; 
      <a href="README.zh-CN.md">🇨🇳 简体中文</a> &nbsp;|&nbsp; 
      <a href="README.ja.md">🇯🇵 日本語</a> &nbsp;|&nbsp; 
      <a href="README.ko.md">🇰🇷 한국어</a> &nbsp;|&nbsp; 
      <a href="README.de.md">🇩🇪 Deutsch</a> &nbsp;|&nbsp; 
      <a href="README.es.md">🇪🇸 <b>Español</b></a>
    </p>
  </details>
</div>

<p align="center">
  <a href="https://novamindlab.github.io/VidNest/">🌐 Portal Oficial y Centro de Descargas</a> •
  <a href="../../wiki/README.md">📚 Documentación de Arquitectura</a> •
  <a href="https://github.com/NovaMindLab/VidNest/releases/latest">⚡ Descargar Última Versión</a>
</p>

---

## 🌟 ¿Por qué VidNest?

**VidNest** es una aplicación de escritorio **Local-First**, ligera y de alto rendimiento, diseñada para creadores de video, investigadores y aficionados al contenido audiovisual.

### 🚀 Características Principales

* 🧠 **Centro de Inteligencia de Video por IA**: Integra algoritmos locales de PNL sin configuración previa, junto con modelos de lenguaje avanzados (OpenAI, DeepSeek, Ollama) y reconocimiento de voz Whisper. Genera resúmenes, puntos clave y etiquetas automáticamente.
* ⚡ **Búsqueda Instantánea FTS5**: Motor de búsqueda de texto completo SQLite FTS5 integrado. Encuentra títulos, etiquetas y diálogos de voz en milisegundos con salto temporal directo.
* 🔍 **Deduplicación Inteligente Multidimensional**: Identifica videos duplicados comparando hashes SHA-256 parciales, resolución, tasa de bits y nombres de archivo. Libera almacenamiento de forma segura.
* 🎵 **Extracción de Audio de Alta Fidelidad**: Compatible con MP3, M4A, WAV y FLAC. La copia de flujo sin pérdida (Stream Copy) extrae el audio en milisegundos sin recodificación.
* 🌐 **Centro de Descargas yt-dlp Integrado**: Guarda videos y audios de YouTube, TikTok, X, Instagram y más de 1.000 plataformas directamente en tu equipo.

---

## 🛠️ Tecnologías

* **Núcleo**: Tauri v2, Rust 2021
* **Base de Datos**: SQLite3 (con extensión FTS5 activada)
* **Interfaz de Usuario**: Vue 3.5, TypeScript, Vite
* **Procesamiento Multimedia**: FFmpeg, FFprobe, Whisper, yt-dlp

---

## 📄 Licencia

Este proyecto está licenciado bajo la [Licencia MIT](../../LICENSE).
