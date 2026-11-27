#!/usr/bin/env bash
# Package an existing Velnosub APK as an unsigned Android App Bundle.
set -euo pipefail

APK=${1:?APK yolu gerekli}
AAB=${2:?AAB çıktı yolu gerekli}
AAPT2=${AAPT2:?Google Maven AAPT2 ikilisinin yolu gerekli}
BUNDLETOOL_JAR=${BUNDLETOOL_JAR:?bundletool JAR yolu gerekli}
test -r "$APK"
test -x "$AAPT2"
test -r "$BUNDLETOOL_JAR"

EXTRA_APK=${3:-}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
"$AAPT2" convert --output-format proto -o "$WORK/proto.apk" "$APK"
PROTO_APKS=("$WORK/proto.apk")
if [ -n "$EXTRA_APK" ]; then
    test -r "$EXTRA_APK"
    "$AAPT2" convert --output-format proto -o "$WORK/extra.apk" "$EXTRA_APK"
    PROTO_APKS+=("$WORK/extra.apk")
fi
python3 - "$WORK/base.zip" "${PROTO_APKS[@]}" <<'PYBUNDLE'
import re
import sys
import zipfile

required = ("manifest/AndroidManifest.xml", "resources.pb", "dex/classes.dex")
documents = {}
with zipfile.ZipFile(sys.argv[1], "w", compression=zipfile.ZIP_DEFLATED) as base:
    for path in sys.argv[2:]:
        with zipfile.ZipFile(path) as apk:
            for source in apk.namelist():
                if source.endswith("/"):
                    continue
                if source == "AndroidManifest.xml":
                    target = "manifest/AndroidManifest.xml"
                elif re.fullmatch(r"classes[0-9]*\.dex", source):
                    target = "dex/" + source
                elif source == "resources.pb" or source.startswith(("res/", "lib/", "assets/")):
                    target = source
                else:
                    continue
                data = apk.read(source)
                if target in documents:
                    if documents[target] != data:
                        raise SystemExit(f"APK sürümleri uyuşmuyor: {target}")
                else:
                    documents[target] = data
                    base.writestr(target, data)
    for name in required:
        if name not in documents:
            raise SystemExit(f"Eksik paket girdisi: {name}")
PYBUNDLE
java -jar "$BUNDLETOOL_JAR" build-bundle --modules="$WORK/base.zip" --output="$WORK/app.aab"
java -jar "$BUNDLETOOL_JAR" validate --bundle="$WORK/app.aab"
cp "$WORK/app.aab" "$AAB"
