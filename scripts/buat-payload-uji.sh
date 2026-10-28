#!/usr/bin/env bash
# Bikin ISO kecil berisi binary + data, untuk menguji wizard di VM RakuOS
# minimal tanpa perlu jaringan atau build tool di dalam VM.
#
# Pakai:
#   ./scripts/buat-payload-uji.sh                    # output/rakuos-bootstrap-payload.iso
#   ./scripts/buat-payload-uji.sh output/uji-2.iso   # nama lain, kalau yang lama
#                                                    # sedang dipasang di VM
#
# Di VM: pasang ISO itu sebagai CD-ROM kedua, lalu
#   sudo mount -o ro /dev/sr1 /mnt
#   /mnt/jalankan.sh --dry-run
#   /mnt/jalankan.sh            # sungguhan, setelah dry-run terlihat benar

set -euo pipefail

cd "$(dirname "$(readlink -f "$0")")/.."

cargo build --release

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

install -Dm755 target/release/rakuos-bootstrap "$work/rakuos-bootstrap"
cp -r data "$work/data"
install -Dm644 data/rakuos-bootstrap.service "$work/rakuos-bootstrap.service"

cat > "$work/jalankan.sh" <<'EOF'
#!/usr/bin/env bash
# Jalankan wizard langsung dari ISO payload (read-only).
set -euo pipefail
mnt="$(cd "$(dirname "$0")" && pwd)"
exec sudo "$mnt/rakuos-bootstrap" --base-dir "$mnt/data" "$@"
EOF
chmod 0755 "$work/jalankan.sh"

iso="${1:-output/rakuos-bootstrap-payload.iso}"
mkdir -p "$(dirname "$iso")"
xorriso -as mkisofs -quiet -R -J -V RAKUBOOT -o "$iso" "$work"
( cd "$(dirname "$iso")" && sha256sum "$(basename "$iso")" > "$(basename "$iso").sha256" )

ls -lh "$iso"
echo "Pasang sebagai CD-ROM kedua di VM, lalu di dalam VM:"
echo "  sudo mount -o ro /dev/sr1 /mnt"
echo "  /mnt/jalankan.sh --dry-run"
