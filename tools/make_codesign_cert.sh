#!/usr/bin/env bash
# Creates the self-signed code signing certificate.
#
#   tools/make_codesign_cert.sh <out-dir>
#
# Writes to <out-dir> (keep it out of the repository):
#   codesign.pfx         - private key + certificate (PKCS#12, password-protected)
#   codesign.pfx.base64  - the same, base64-encoded: the CODESIGN_PFX secret
#   codesign.password    - the PFX password: the CODESIGN_PASSWORD secret
# and the public certificate to assets/codesign.cer (DER), which users install.
set -euo pipefail

out=${1:?usage: $0 <out-dir>}
repo=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$out"
umask 077

openssl req -x509 -newkey rsa:3072 -sha256 -days 3650 -nodes \
  -keyout "$out/codesign.key" -out "$out/codesign.pem" \
  -subj "/CN=Mind PKM Code Signing" \
  -addext "basicConstraints=critical,CA:FALSE" \
  -addext "keyUsage=critical,digitalSignature" \
  -addext "extendedKeyUsage=codeSigning" \
  -addext "subjectKeyIdentifier=hash"

openssl rand -base64 24 | tr -d '\n' > "$out/codesign.password"
openssl pkcs12 -export -name "Mind PKM Code Signing" \
  -inkey "$out/codesign.key" -in "$out/codesign.pem" \
  -keypbe AES-256-CBC -certpbe AES-256-CBC -macalg sha256 \
  -passout "file:$out/codesign.password" -out "$out/codesign.pfx"
base64 -w0 "$out/codesign.pfx" > "$out/codesign.pfx.base64"
rm "$out/codesign.key"

openssl x509 -in "$out/codesign.pem" -outform DER -out "$repo/assets/codesign.cer"
openssl x509 -in "$out/codesign.pem" -noout -subject -enddate -fingerprint -sha1
