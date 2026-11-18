#!/bin/sh
# Run the ORIGINAL Loquendo 7 engine on a text and record each stage's output.
#
#   oracle/run.sh <voice> <utf-8 text file> <output prefix>
#
# First use of a voice installs its installer into oracle/prefix (LOQ_PREFIX
# for another), a private Wine prefix (nothing outside it is touched). Inno
# Setup installers run under Wine; the Wise/MSI ones (voice files only) are
# unpacked by `quoquendo --unpack` over an engine an Inno voice installed
# (Raffaele's, installed first when missing). The licence is put into that
# prefix's registry whenever it changes or after an install (installers
# overwrite it):
#  - by default the one loquendo-keygen made for this computer and Wine user
#    `wineuser` (LOQ_LICENCE to use another): the engine runs as it is;
#  - without it, the loquendo-tts-server licence (from its git history), bound
#    to the network card 7A:8B:9C:1D:2E:3F: the engine then runs in its own
#    network namespace holding a dummy card with that address (no root).
# Writes <prefix>.fon/.ev/.wav (see loqref.c).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
VOICE=$1 TEXT=$2 OUT=$3
INSTALLERS=${LOQ_INSTALLERS:-"$HOME/Downloads/BitTorrent/text-to-speech-oddcast-voices-pack-2024/Loquendo 7"}
LICENCE=${LOQ_LICENCE:-"$HERE/../license-wineuser.reg"}
MAC=7A:8B:9C:1D:2E:3F

export WINEPREFIX="${LOQ_PREFIX:-$HERE/prefix}" WINEDEBUG=-all USER=wineuser
unset DISPLAY WAYLAND_DISPLAY
LTTS7="$WINEPREFIX/drive_c/Program Files (x86)/Loquendo/LTTS7"

install_voice() {
    local INSTALLER i
    # installers are named "<Voice> - <Language>.exe" (Roberto ships as Raffaele)
    case "$1" in
    Roberto) INSTALLER="Raffaele - Italian.exe" ;;
    Linlin) INSTALLER="LinLin - Chinese.exe" ;;
    *) INSTALLER=${LOQ_INSTALLER:-$(cd "$INSTALLERS" && ls "$1 - "*.[eE][xX][eE] 2>/dev/null | head -1)} ;;
    esac
    [ -n "$INSTALLER" ] || { echo "no installer for $1 in $INSTALLERS" >&2; exit 1; }
    echo "installing $1 into $WINEPREFIX (one time)..." >&2
    if LC_ALL=C grep -qaF "$(printf 'rDlPtS\315\346\327{\013*')" "$INSTALLERS/$INSTALLER"; then
        # Inno Setup wants a display even when silent: give it a virtual one
        xvfb-run -a wine "$INSTALLERS/$INSTALLER" /VERYSILENT /SUPPRESSMSGBOXES >/dev/null 2>&1 &
        # the installer ends by starting the dongle registration tool: stop it
        # there (matched by process name; -f would match this script's caller)
        i=0
        while ! pkill -i REGVOICE 2>/dev/null; do
            i=$((i + 1)); [ $i -lt 600 ] || { echo "installer did not finish" >&2; exit 1; }
            sleep 1
        done
        wait || true
        wineserver -w
    else
        # Wise/MSI: voice files only, laid over an installed engine
        [ -f "$WINEPREFIX/.voice-Roberto" ] || install_voice Roberto
        QQ="$HERE/../target/release/quoquendo"
        [ -x "$QQ" ] || cargo build --release --bin quoquendo --manifest-path "$HERE/../Cargo.toml" >&2
        "$QQ" --unpack "$INSTALLERS/$INSTALLER" "$LTTS7" >&2
    fi
    touch "$WINEPREFIX/.voice-$1"
    rm -f "$WINEPREFIX/.licence"
}
[ -f "$WINEPREFIX/.voice-$VOICE" ] || install_voice "$VOICE"

# the licence in force, and whether it needs the dummy network card
lic=$(mktemp)
if [ -f "$LICENCE" ]; then
    cp "$LICENCE" "$lic"
    NETNS=0
else
    git -C "$HOME/ghq/github.com/depau/loquendo-tts-server" show e44e1e9:docker/license.reg >"$lic"
    NETNS=1
fi
sum=$(sha256sum <"$lic" | cut -d' ' -f1)
if [ "$(cat "$WINEPREFIX/.licence" 2>/dev/null)" != "$sum" ]; then
    wineserver -w
    wine reg import "$lic" >/dev/null 2>&1
    wineserver -w
    echo "$sum" >"$WINEPREFIX/.licence"
fi
rm -f "$lic"

BIN="$LTTS7/bin"
cp "$HERE/loqref.exe" "$WINEPREFIX/drive_c/loqref.exe"
TEXT_W=$(winepath -w "$TEXT")
OUT_W=$(winepath -w "$OUT")
BIN_W=$(winepath -w "$BIN")
if [ "$NETNS" = 0 ]; then
    wine 'C:\loqref.exe' "$BIN_W" "$VOICE" "$TEXT_W" "$OUT_W"
    wineserver -k 2>/dev/null || true
    exit 0
fi
# own network namespace: only lo and the licensed dummy card exist in it
exec unshare -rn sh -c "
ip link add dummy0 type dummy && ip link set dev dummy0 address $MAC && ip link set dummy0 up &&
wine 'C:\\loqref.exe' '$BIN_W' '$VOICE' '$TEXT_W' '$OUT_W'; wineserver -k 2>/dev/null; true"
