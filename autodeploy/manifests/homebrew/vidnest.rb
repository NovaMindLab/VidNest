cask "vidnest" do
  version "1.1.0"
  sha256 "e8def2130bf0bc30da162033674a7ffb9ba421f3b5ae311c60adac3ff0c0afff"

  url "https://github.com/NovaMindLab/VidNest/releases/download/v1.1.0/Local.Video.Intelligence_1.1.0_universal.dmg"
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
