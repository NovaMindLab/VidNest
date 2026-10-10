cask "vidnest" do
  version "1.0.0"
  sha256 "10b449d1cf4e841d067ef0d8dd73317c16e7f4f177e9cfd12ef263fe959fc6e9"

  url "https://github.com/NovaMindLab/VidNest/releases/download/v1.0.0/Local.Video.Intelligence_1.0.0_universal.dmg"
  name "VidNest"
  name "Local Video Intelligence"
  desc "Local-first video analysis, indexing, and intelligent summarization desktop app."
  homepage "https://github.com/NovaMindLab/VidNest"

  livecheck do
    url :url
    strategy :github_latest
  end

  app "VidNest.app"

  postflight do
    # Remove macOS quarantine bit to prevent Sequoia Gatekeeper damage prompt
    system_command "/usr/bin/xattr",
                   args: ["-rd", "com.apple.quarantine", "#{appdir}/VidNest.app"],
                   sudo: false
  end

  zap trash: [
    "~/Library/Application Support/com.vidnest.localvideointelligence",
    "~/Library/Preferences/com.vidnest.localvideointelligence.plist",
    "~/Library/Saved Application State/com.vidnest.localvideointelligence.savedState",
  ]
end
