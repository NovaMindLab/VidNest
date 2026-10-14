# 🎬 VidNest - 로컬 우선 AI 비디오 인텔리전스 & 미디어 허브

<p align="center">
  <img src="../../src-tauri/icons/icon.png" width="128" height="128" alt="VidNest Logo" />
</p>

<p align="center">
  <strong>차세대 오픈소스 데스크톱 비디오 인텔리전스, 콘텐츠 이해 및 자산 관리 워크스페이스</strong><br>
  100% 프라이빗 & 로컬 우선 · AI 요약 및 토픽 타임라인 · 밀리초 FTS5 전체 검색 · 독립 PIP 플레이어 · 크로스 플랫폼 데스크톱 네이티브
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
    <summary>🌐 <b>언어 선택 / Language / 言語 / 언어 / Sprache / Idioma (6 Languages)</b></summary>
    <br/>
    <p>
      <a href="../../README.md">🇺🇸 English</a> &nbsp;|&nbsp; 
      <a href="../../README.zh-CN.md">🇨🇳 简体中文</a> &nbsp;|&nbsp; 
      <a href="README.ja.md">🇯🇵 日本語</a> &nbsp;|&nbsp; 
      <a href="README.ko.md">🇰🇷 <b>한국어</b></a> &nbsp;|&nbsp; 
      <a href="README.de.md">🇩🇪 Deutsch</a> &nbsp;|&nbsp; 
      <a href="README.es.md">🇪🇸 Español</a>
    </p>
  </details>
</div>

<p align="center">
  <a href="https://novamindlab.github.io/VidNest/">🌐 공식 포털 & 다운로드 센터</a> •
  <a href="../../wiki/README.md">📚 개발자 아키텍처 문서</a> •
  <a href="https://github.com/NovaMindLab/VidNest/releases/latest">⚡ 최신 버전 다운로드</a>
</p>

---

## 🌟 VidNest 소개

**VidNest**는 영상 제작자, 연구원, 수집가를 위한 고성능, 초경량 **로컬 우선 AI 비디오 인텔리전스 및 미디어 관리 데스크톱 도구**입니다.

### 🚀 핵심 기능

* 🧠 **AI 비디오 인텔리전스 허브**: 무설정 로컬 NLP 알고리즘과 OpenAI / DeepSeek / Ollama 등 외부 대규모 언어 모델(LLM), Whisper 음성 인식을 융합하여 영상 핵심 요약, 타임라인 분석, 스마트 태그를 자동 생성합니다.
* ⚡ **밀리초급 FTS5 전체 검색**: 비디오 제목, 포맷, 태그뿐만 아니라 Whisper 음성 인식 대사까지 밀리초 단위로 검색하고 해당 타임스탬프로 즉시 이동하여 재생합니다.
* 🔍 **다차원 비디오 중복 검출 및 정리**: SHA-256 청크 샘플링, 해상도, 비트레이트, 파일명 유사도 등을 종합 분석하여 중복 비디오를 안전하게 병합 정리하고 저장 공간을 확보합니다.
* 🎵 **고음질 오디오 추출**: MP3, M4A, WAV, FLAC 포맷 지원. 무손실 스트림 복사(Stream Copy)를 통해 품질 저하 없이 0초 만에 배경음악과 음성을 분리 저장합니다.
* 🌐 **내장 yt-dlp 다운로드 센터**: YouTube, TikTok, X, Instagram 등 1,000개 이상의 사이트에서 고화질 영상을 손쉽게 다운로드합니다.

---

## 🛠️ 기술 스택

* **엔진 & 코어**: Tauri v2, Rust 2021
* **데이터베이스**: SQLite3 (FTS5 전문 검색 활성화)
* **프론트엔드 UI**: Vue 3.5, TypeScript, Vite
* **미디어 파이프라인**: FFmpeg, FFprobe, Whisper, yt-dlp

---

## 📄 라이선스

본 프로젝트는 [MIT License](../../LICENSE)에 따라 오픈소스로 배포됩니다.
