#!/usr/bin/env bash
# braid installer script
# usage: curl -sSL https://raw.githubusercontent.com/alextes/braid/main/install.sh | bash

set -euo pipefail

REPO="alextes/braid"
BINARY_NAME="brd"
INSTALL_DIR="${INSTALL_DIR:-${CARGO_HOME:-$HOME/.cargo}/bin}"

# detect platform
detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="unknown-linux-gnu" ;;
        Darwin*) os="apple-darwin" ;;
        *)       echo "error: unsupported OS: $(uname -s)"; exit 1 ;;
    esac

    case "$(uname -m)" in
        x86_64)  arch="x86_64" ;;
        aarch64) arch="aarch64" ;;
        arm64)   arch="aarch64" ;;
        *)       echo "error: unsupported architecture: $(uname -m)"; exit 1 ;;
    esac

    echo "${arch}-${os}"
}

# get latest release version
get_latest_version() {
    curl -sSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name":' \
        | sed -E 's/.*"([^"]+)".*/\1/'
}

# main
main() {
    echo "installing braid..."

    local platform version download_url tmp_dir

    platform=$(detect_platform)
    version=$(get_latest_version)

    if [[ -z "$version" ]]; then
        echo "error: could not determine latest version"
        echo ""
        echo "no releases found. install from source instead:"
        echo "  cargo install --git https://github.com/${REPO}.git"
        exit 1
    fi

    echo "  version:  ${version}"
    echo "  platform: ${platform}"

    # cargo-dist naming convention: <name>-<target>.tar.xz
    download_url="https://github.com/${REPO}/releases/download/${version}/braid-${platform}.tar.xz"

    echo "  downloading from: ${download_url}"

    tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT

    if ! curl -sSL "$download_url" -o "${tmp_dir}/braid.tar.xz"; then
        echo "error: download failed"
        echo ""
        echo "the release may not have prebuilt binaries yet."
        echo "install from source instead:"
        echo "  cargo install --git https://github.com/${REPO}.git"
        exit 1
    fi

    tar -xJf "${tmp_dir}/braid.tar.xz" -C "$tmp_dir"

    # ensure install directory exists
    mkdir -p "$INSTALL_DIR"

    # install binary - cargo-dist extracts to braid-<target>/ subdirectory
    local binary_path
    if [[ -f "${tmp_dir}/braid-${platform}/${BINARY_NAME}" ]]; then
        binary_path="${tmp_dir}/braid-${platform}/${BINARY_NAME}"
    elif [[ -f "${tmp_dir}/${BINARY_NAME}" ]]; then
        binary_path="${tmp_dir}/${BINARY_NAME}"
    else
        echo "error: could not find binary in archive"
        echo "contents of tmp_dir:"
        ls -la "$tmp_dir"
        exit 1
    fi

    mv "$binary_path" "${INSTALL_DIR}/${BINARY_NAME}"

    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

    echo ""
    echo "installed ${BINARY_NAME} to ${INSTALL_DIR}/${BINARY_NAME}"
    echo ""

    # check if install dir is in PATH
    if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
        echo "note: ${INSTALL_DIR} is not in your PATH"
        echo "add it with:"
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    else
        echo "run 'brd --help' to get started"
    fi
}

main "$@"
