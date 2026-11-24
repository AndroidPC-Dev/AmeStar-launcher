#!/usr/bin/env python3
"""Puts the Starling engine into the Amethyst-based app (amethyst/app_pojavlauncher) before Gradle builds it.

    python3 packaging/android/stage_gradle.py [--profile quick] [--no-build]

It builds the game library with build_apk.py's own steps, then copies into the Gradle module:
the game library and the helper libraries (jniLibs/arm64-v8a), what the setup needs from assets/,
Starling's Java classes (with `R` pointed at the app's own) and its resources.
"""
import argparse
import os
import re
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import build_apk as b  # noqa: E402

APP = b.ROOT / "amethyst" / "app_pojavlauncher" / "src" / "main"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", default="quick")
    parser.add_argument("--no-build", action="store_true")
    args = parser.parse_args()
    abi = b.ABI
    triple = b.TARGETS[abi][0]
    sdk = b.sdk_dir()
    ndk = Path(os.environ.get("ANDROID_NDK_HOME") or sdk / "ndk" / b.NDK)
    if not args.no_build:
        b.build_library(args.profile, ndk, sdk, abi)
    built = b.ROOT / "target" / triple / ("debug" if args.profile == "dev" else args.profile) / f"lib{b.LIBRARY}.so"
    if not built.is_file():
        sys.exit(f"stage_gradle: {built} not built")
    out = b.ROOT / "target" / "android"
    out.mkdir(parents=True, exist_ok=True)
    library = out / f"lib{b.LIBRARY}.so"
    b.run(b.ndk_tool(ndk, "llvm-strip"), "--strip-unneeded", "-o", library, built)

    libs = APP / "jniLibs" / abi
    libs.mkdir(parents=True, exist_ok=True)
    libraries = b.build_launchers(ndk, out, abi)
    libraries.update(b.build_adrenotools(ndk, out, abi))
    libraries[library.name] = library
    libraries.update(b.rust_dylibs(ndk, built, out))
    for name, path in libraries.items():
        shutil.copyfile(path, libs / name)
        print("lib", name, Path(path).stat().st_size)

    assets = b.assets(out, abi)
    shutil.copytree(assets, APP / "assets", dirs_exist_ok=True)

    java = APP / "java" / "io" / "github" / "juanluci" / "starling"
    shutil.rmtree(java, ignore_errors=True)
    java.mkdir(parents=True)
    for source in sorted((HERE / "java" / "io" / "github" / "juanluci" / "starling").glob("*.java")):
        text = source.read_text(encoding="utf-8")
        if re.search(r"\bR\.(string|color|style|attr)\.", text) and "import net.kdt.pojavlaunch.R;" not in text:
            text = text.replace("\nimport ", "\nimport net.kdt.pojavlaunch.R;\nimport ", 1)
        (java / source.name).write_text(text, encoding="utf-8")

    res = APP / "res"
    for folder in ("values", "values-v31"):
        for source in sorted((HERE / "res" / folder).glob("*.xml")):
            target = res / folder
            target.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target / f"starling_{source.name}")
    theme = (b.ROOT / "crates/horizon-theme/src/lib.rs").read_text(encoding="utf-8")
    dark = theme.split("pub const DARK: Palette = Palette {", 1)[1].split("};", 1)[0]
    colors = dict(re.findall(r"(\w+): 0x([0-9a-fA-F]{6})", dark))
    entries = "\n".join(
        f'<color name="starling_{name}">#{colors[name]}</color>'
        for name in ("page", "rule", "ink", "ink_soft", "accent", "on_accent")
    )
    (res / "values" / "starling_colors.xml").write_text(f"<resources>\n{entries}\n</resources>\n", encoding="utf-8")
    print("staged")


if __name__ == "__main__":
    main()
