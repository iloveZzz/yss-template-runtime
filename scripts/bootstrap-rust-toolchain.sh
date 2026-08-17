#!/usr/bin/env sh
set -eu

runtime_root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
runtime_cargo_home="$runtime_root/.tooling/cargo"
runtime_rustup_home="$runtime_root/.tooling/rustup"
runtime_toolchain="1.97.1"
runtime_init_version="1.29.0"
runtime_init_sha256="aeb4105778ca1bd3c6b0e75768f581c656633cd51368fa61289b6a71696ac7e1"
runtime_init_url="https://static.rust-lang.org/rustup/archive/$runtime_init_version/aarch64-apple-darwin/rustup-init"
runtime_init_dir="$(mktemp -d)"
runtime_init_bin="$runtime_init_dir/rustup-init"

cleanup() {
  rm -rf "$runtime_init_dir"
}
trap cleanup EXIT INT TERM

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) ;;
  *)
    echo "此 bootstrap 当前只支持 macOS arm64；其他目标由后续 CI 合同负责" >&2
    exit 2
    ;;
esac

mkdir -p "$runtime_cargo_home" "$runtime_rustup_home"
curl -fsSL --proto '=https' --tlsv1.2 "$runtime_init_url" -o "$runtime_init_bin"
echo "$runtime_init_sha256  $runtime_init_bin" | shasum -a 256 -c -
chmod 755 "$runtime_init_bin"
CARGO_HOME="$runtime_cargo_home" RUSTUP_HOME="$runtime_rustup_home" "$runtime_init_bin" -y --profile minimal --default-toolchain none --no-modify-path

if [ -d "$runtime_rustup_home/toolchains/$runtime_toolchain-aarch64-apple-darwin" ]; then
  CARGO_HOME="$runtime_cargo_home" RUSTUP_HOME="$runtime_rustup_home" "$runtime_cargo_home/bin/rustup" toolchain uninstall "$runtime_toolchain"
fi

CARGO_HOME="$runtime_cargo_home" RUSTUP_HOME="$runtime_rustup_home" "$runtime_cargo_home/bin/rustup" toolchain install "$runtime_toolchain" --profile minimal --component rustfmt --component clippy
CARGO_HOME="$runtime_cargo_home" RUSTUP_HOME="$runtime_rustup_home" "$runtime_cargo_home/bin/rustup" default "$runtime_toolchain"
CARGO_HOME="$runtime_cargo_home" RUSTUP_HOME="$runtime_rustup_home" "$runtime_cargo_home/bin/cargo" --version
