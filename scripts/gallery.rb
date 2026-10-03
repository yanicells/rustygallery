# frozen_string_literal: true

cask "rusty-gallery" do
  version "0.1.0"
  sha256 :no_check

  url "https://github.com/yanicells/rustygallery/releases/download/v#{version}/gallery.app.zip"
  name "gallery"
  desc "Minimal photo and video viewer"
  homepage "https://github.com/yanicells/rustygallery"

  app "gallery.app"

  caveats <<~EOS
    Until a GitHub release exists, build a local app instead:

      ./scripts/bundle-macos.sh
      open dist/gallery.app
  EOS
end
