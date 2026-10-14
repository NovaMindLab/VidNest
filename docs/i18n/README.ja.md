# 🎬 VidNest - ローカルファースト AI 動画インテリジェンス＆メディアハブ

<p align="center">
  <img src="../../src-tauri/icons/icon.png" width="128" height="128" alt="VidNest Logo" />
</p>

<p align="center">
  <strong>次世代オープンソース デスクトップ動画インテリジェンス・コンテンツ理解・資産管理ワークスペース</strong><br>
  100% プライベート＆ローカルファースト · AI エグゼクティブサマリー＆トピックタイムライン · ミリ秒単位の全文検索 · 独立したピクチャーインピクチャー · クロスプラットフォームネイティブ
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
    <summary>🌐 <b>言語切替 / Language / 言語 / 언어 / Sprache / Idioma (6 Languages)</b></summary>
    <br/>
    <p>
      <a href="../../README.md">🇺🇸 English</a> &nbsp;|&nbsp; 
      <a href="../../README.zh-CN.md">🇨🇳 简体中文</a> &nbsp;|&nbsp; 
      <a href="README.ja.md">🇯🇵 <b>日本語</b></a> &nbsp;|&nbsp; 
      <a href="README.ko.md">🇰🇷 한국어</a> &nbsp;|&nbsp; 
      <a href="README.de.md">🇩🇪 Deutsch</a> &nbsp;|&nbsp; 
      <a href="README.es.md">🇪🇸 Español</a>
    </p>
  </details>
</div>

<p align="center">
  <a href="https://novamindlab.github.io/VidNest/">🌐 公式ポータル＆ダウンロード</a> •
  <a href="../../wiki/README.md">📚 開発者ドキュメント</a> •
  <a href="https://github.com/NovaMindLab/VidNest/releases/latest">⚡ 最新リリースをダウンロード</a>
</p>

---

## 🌟 なぜ VidNest なのか？

**VidNest** は、クリエイター、研究者、動画愛好家のために設計された、高機能で軽量な**ローカルファースト AI 動画インテリジェンス・管理デスクトップアプリケーション**です。

### 🚀 主な機能

* 🧠 **AI 統合インテリジェンス中枢 (AI Intelligence Hub)**: ゼロ設定のローカル NLP アルゴリズムに加え、OpenAI/DeepSeek/Ollama などの大規模言語モデルと Whisper 音声認識を融合。動画の内容要約、見どころタイムライン、スマートタグを完全自動生成。
* ⚡ **ミリ秒級 FTS5 全文検索**: SQLite FTS5 全文検索エンジンを統合。動画タイトル、タグ、さらには AI 音声認識による台詞までミリ秒単位で高速検索。
* 🔍 **多次元スマート重複動画整理**: SHA-256 サンプリングハッシュ、解像度、ビットレート、類似ファイル名などを組み合わせたインテリジェント重複判定。重複ファイルを安全に整理し、空き容量を解放。
* 🎵 **Hi-Fi 高音質音声抽出**: MP3、M4A、WAV、FLAC への抽出に対応。無劣化ストリームコピー（Stream Copy）により、品質劣化なしで一瞬で音声を分離保存。
* 🌐 **組み込み yt-dlp ダウンロードセンター**: YouTube、TikTok、X (Twitter)、Instagram など 1000 以上のプラットフォームから高品質動画・音声をローカル保存。

---

## 🛠️ 技術スタック

* **コア基盤**: Tauri v2, Rust 2021
* **データベース**: SQLite3 (FTS5 全文検索拡張有効)
* **UI フレームワーク**: Vue 3.5, TypeScript, Vite
* **マルチメディア処理**: FFmpeg / FFprobe, Whisper, yt-dlp

---

## 📄 ライセンス

本プロジェクトは [MIT License](../../LICENSE) のもとでオープンソースとして公開されています。
