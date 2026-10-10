cask "vidnest" do
  version "1.1.0"
  sha256 "bf465f99eae365f1d6a9cfc457ef10fe13d5b834d8ada530d86237b7963277c4"

  url "https://github.com/NovaMindLab/VidNest/releases/download/v1.1.0/Local.Video.Intelligence_1.1.0_aarch64.dmg"
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
