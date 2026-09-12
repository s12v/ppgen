#!/usr/bin/env bash
# Renders the Homebrew formula for s12v/homebrew-tap from a release's SHA256SUMS.
# Usage: render-formula.sh <version> <path/to/SHA256SUMS> > Formula/ppgen.rb
set -euo pipefail

version=$1
sums=$2
base="https://github.com/s12v/ppgen/releases/download/v${version}"

sha() {
  local hash
  hash=$(awk -v f="ppgen-$1.tar.gz" '$2 == f { print $1 }' "$sums")
  [ -n "$hash" ] || { echo "render-formula: no checksum for $1 in $sums" >&2; exit 1; }
  echo "$hash"
}

cat <<RUBY
class Ppgen < Formula
  desc "Random, easy-to-remember passphrases from the EFF wordlist"
  homepage "https://github.com/s12v/ppgen"
  license "MIT"

  on_macos do
    on_arm do
      url "${base}/ppgen-aarch64-apple-darwin.tar.gz"
      sha256 "$(sha aarch64-apple-darwin)"
    end
    on_intel do
      url "${base}/ppgen-x86_64-apple-darwin.tar.gz"
      sha256 "$(sha x86_64-apple-darwin)"
    end
  end

  on_linux do
    on_arm do
      url "${base}/ppgen-aarch64-unknown-linux-musl.tar.gz"
      sha256 "$(sha aarch64-unknown-linux-musl)"
    end
    on_intel do
      url "${base}/ppgen-x86_64-unknown-linux-musl.tar.gz"
      sha256 "$(sha x86_64-unknown-linux-musl)"
    end
  end

  def install
    bin.install "ppgen"
  end

  test do
    assert_match(/\A[a-z-]+\n\z/, shell_output("#{bin}/ppgen -w 3"))
  end
end
RUBY
