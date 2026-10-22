cask "vidnest" do
  version "2.9.6"
  sha256 "7145214162b4f942ed0fd7fe41e1ca561046faeb6a631f8ccf3d337c94bafe76"

  url "https://github.com/NovaMindLab/VidNest/releases/download/v2.9.6/Local.Video.Intelligence_2.9.6_universal.dmg"
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
