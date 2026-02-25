class Todo < Formula
  desc "Human-Agent Task Coordination Protocol - CLI task management for AI agents"
  homepage "https://github.com/JhihJian/SUMM-Todo"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/JhihJian/SUMM-Todo/releases/download/v#{version}/todo-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
    on_arm do
      url "https://github.com/JhihJian/SUMM-Todo/releases/download/v#{version}/todo-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/JhihJian/SUMM-Todo/releases/download/v#{version}/todo-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
    on_arm do
      url "https://github.com/JhihJian/SUMM-Todo/releases/download/v#{version}/todo-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
  end

  def install
    bin.install "todo"
  end

  test do
    system "#{bin}/todo", "--help"
  end
end
