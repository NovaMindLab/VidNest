cask "vidnest" do
  version "0.1.0"
  sha256 "1a327884351362d9a2a0f79d1bd5e7aa5554119f90f0ef5e59d699257d680a3c"

  url "https://github.com/NovaMindLab/VidNest/releases/download/v0.1.0/Local.Video.Intelligence_0.1.0_universal.dmg"
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
