#!/usr/bin/env python3
"""Build one release archive, deterministically (stdlib only, works on Linux, macOS and Windows).

    package_release.py --target T --binary PATH --version V --out DIR [--extras DIR]

Creates DIR/opshero-V-T.tar.gz (a .zip for a Windows target) holding, under the directory
opshero-V-T/, the binary, the LICENSE, NOTICE and README.md files and every file under --extras
(see gen_extras.py), and DIR/<archive>.sha256 with one line in the `sha256sum` format. The archive
is reproducible: entries are sorted, timestamps are zero, ownership is neutral and the compression
header carries no name or time, so the same inputs always give the same bytes (and checksum).
"""
import argparse, gzip, hashlib, io, os, sys, tarfile, zipfile

FILES = ["LICENSE", "NOTICE", "README.md"]


def extra_files(extras):
    found = []
    for base, _, files in os.walk(extras):
        for f in files:
            path = os.path.join(base, f)
            found.append((os.path.relpath(path, extras).replace(os.sep, "/"), path))
    return found


def build(target, binary, version, out_dir, root, extras=None):
    name = f"opshero-{version}-{target}"
    top = name
    windows = "windows" in target
    exe = "opshero.exe" if windows else "opshero"
    entries = [(f"{top}/{exe}", binary, 0o755)] + [(f"{top}/{f}", os.path.join(root, f), 0o644) for f in FILES]
    entries += [(f"{top}/{rel}", path, 0o644) for rel, path in extra_files(extras)] if extras else []
    entries.sort(key=lambda e: e[0])
    os.makedirs(out_dir, exist_ok=True)
    if windows:
        archive = os.path.join(out_dir, f"{name}.zip")
        with zipfile.ZipFile(archive, "w") as zf:
            for arcname, path, mode in entries:
                with open(path, "rb") as f:
                    data = f.read()
                info = zipfile.ZipInfo(arcname, date_time=(1980, 1, 1, 0, 0, 0))
                info.external_attr = mode << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                zf.writestr(info, data, compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)
    else:
        raw = io.BytesIO()
        with tarfile.open(fileobj=raw, mode="w", format=tarfile.USTAR_FORMAT) as tar:
            for arcname, path, mode in entries:
                with open(path, "rb") as f:
                    data = f.read()
                info = tarfile.TarInfo(arcname)
                info.size, info.mtime, info.mode = len(data), 0, mode
                info.uid = info.gid = 0
                info.uname = info.gname = ""
                tar.addfile(info, io.BytesIO(data))
        archive = os.path.join(out_dir, f"{name}.tar.gz")
        with open(archive, "wb") as f:
            with gzip.GzipFile(filename="", mode="wb", fileobj=f, mtime=0, compresslevel=9) as gz:
                gz.write(raw.getvalue())
    digest = hashlib.sha256(open(archive, "rb").read()).hexdigest()
    line = f"{digest}  {os.path.basename(archive)}\n"
    with open(archive + ".sha256", "w") as f:
        f.write(line)
    return archive, line


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--target", required=True)
    ap.add_argument("--binary", required=True)
    ap.add_argument("--version", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--extras", help="directory of extra files to ship (see gen_extras.py)")
    a = ap.parse_args()
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    for f in [a.binary] + [os.path.join(root, x) for x in FILES]:
        if not os.path.isfile(f):
            sys.exit(f"missing file: {f}")
    _, line = build(a.target, a.binary, a.version, a.out, root, a.extras)
    sys.stdout.write(line)


if __name__ == "__main__":
    main()
