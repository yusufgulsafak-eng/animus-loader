#!/usr/bin/env python3
"""Build the Loader payload from the user's exact, unchanged source archive.

The payload is supplied locally; translation data and helper binaries are never
added to the source repository. This script never changes a game installation.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import zipfile

MANIFEST_HASH = "ee397ca77e96e0a1c53fff3cf0d26cd58b64a99cdad8c69d9ef34014d87ff6e4"
TARGETS = [
    "DataPC_boot.forge", "DataPC_boot_patch_01.forge", "DataPC_boot_patch_02.forge",
    "dlc_20/DataPC_boot_20_dlc.forge", "dlc_20/DataPC_boot_20_dlc_patch_01.forge",
]
PROFILE = {
    "edition": "ORIJINAL", "minimum_loader_version": "0.1.5",
    "executable": "ACShadows.exe", "process_name": "ACShadows.exe",
    "required_files": TARGETS,
    "install_actions": [{
        "id": "762b942d-58d8-4524-beb7-ae37cce1a6d7", "type": "INSTALL_SHADOWS",
        "source": "MANIFEST_SHA256.txt", "destination": "DataPC_boot.forge", "backup": True,
        "expected_sha256": MANIFEST_HASH, "options": {},
    }],
}
README = """ANIMUS | ASSASSIN'S CREED SHADOWS | ORIJINAL

Bu paket Loader 0.1.5 veya daha yeni sürümde INSTALL_SHADOWS desteği gerektirir.
Loader 0.1.4 ve normal COPY_DIRECTORY işlemi bu paketi kuramaz.

Loader'da oyun klasörünü seçip Yama Kur düğmesini kullanın. Kurulum, 82 çeviri
kaydını işler; beş FORGE dosyasının indekslerini yedekler ve eklenen verileri
kaldırmada geri alır. Kaldırmak için Loader'da Yamayı Kaldır düğmesini kullanın.
Kurulumdan ve kaldırmadan önce oyunu kapatın.

Ana oyunun üç boot arşivi ile dlc_20 klasöründeki iki boot arşivi gereklidir.
Uyumsuz oyun indeksi veya eksik kayıt bulunursa oyun dosyalarına yazılmaz.
Bu ORIJINAL paket, korsan sürümler için doğrulanmış değildir.

data/tools içindeki iki yardımcı EXE ve Oodle DLL, kaynak paketteki halleriyle
korunmuştur. Loader bunları içeriden çalıştırır; elle EXE veya BAT açılmaz.
Paket içinde çalıştırılacak harici BAT veya PowerShell betiği yoktur.

Oyun klasöründeki .animus-shadows-backup ve Loader yedeklerini silmeyin.
Eski bağımsız kurucuyla yüklenmiş bir yama varsa önce onunla kaldırın.
Bu paket bağımsız eski kurulumun yedeğini devralmaz.

Yerel yapısal ve geri alma kontrolleri, oyun içinde çeviri testi değildir.
Oyun içi uyumluluk ve çeviri görünümü ayrıca Windows oyununda denenmelidir.
"""


def write(zf, name, data):
    info = zipfile.ZipInfo(name, (2026, 10, 17, 0, 0, 0))
    info.compress_type = zipfile.ZIP_DEFLATED
    info.external_attr = 0o100644 << 16
    zf.writestr(info, data, compresslevel=9)


def build(source, output):
    if source.resolve() == output.resolve():
        raise ValueError("Output must be separate from the source ZIP")
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(source) as original:
        names = original.namelist()
        if len(names) != len(set(names)):
            raise ValueError("Duplicate source ZIP entries")
        manifest = original.read("MANIFEST_SHA256.txt")
        if hashlib.sha256(manifest).hexdigest() != MANIFEST_HASH:
            raise ValueError("Unsupported source manifest")
        entries = []
        for line in manifest.decode("utf-8-sig").splitlines():
            match = re.fullmatch(r"([a-f0-9]{64})  (data/[A-Za-z0-9_./-]+)", line)
            if not match or any(p in ("", ".", "..") for p in match[2].split("/")):
                raise ValueError("Unsafe source manifest entry")
            entries.append((match[2], match[1]))
        if len(entries) != 1314 or len(set(p for p, _ in entries)) != 1314:
            raise ValueError("Expected 1314 distinct payload files")
        if {n for n in names if n.startswith("data/") and not n.endswith("/")} != {p for p, _ in entries}:
            raise ValueError("Payload files differ from the pinned manifest")
        temporary = output.with_suffix(output.suffix + ".partial")
        try:
            with zipfile.ZipFile(temporary, "w") as payload:
                for path, expected in entries:
                    data = original.read(path)
                    if hashlib.sha256(data).hexdigest() != expected:
                        raise ValueError(f"Source hash mismatch: {path}")
                    write(payload, path, data)
                write(payload, "MANIFEST_SHA256.txt", manifest)
                write(payload, "BENI_OKU.txt", README.encode("utf-8-sig"))
                write(payload, "LOADER_PROFILI.json", (json.dumps(PROFILE, ensure_ascii=False, indent=2) + "\n").encode())
            with zipfile.ZipFile(temporary) as check:
                if check.testzip() is not None:
                    raise ValueError("Output CRC validation failed")
                if sum(n.endswith(".dat.txt") for n in check.namelist()) != 82:
                    raise ValueError("Expected 82 translations")
                if any(n.lower().endswith((".bat", ".ps1")) for n in check.namelist()):
                    raise ValueError("Unexpected external script")
            temporary.replace(output)
        finally:
            temporary.unlink(missing_ok=True)
    with output.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    result = {"file": output.name, "size_bytes": output.stat().st_size, "sha256": digest, **PROFILE}
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    build(args.source, args.output)
