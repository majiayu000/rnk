#!/usr/bin/env python3
"""Release helpers for crates.io package/version checks."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import urllib.error
import urllib.request


EXIT_EXISTS = 0
EXIT_MISSING = 10
EXIT_ERROR = 20


def cargo_metadata() -> dict:
    output = subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
        text=True,
    )
    return json.loads(output)


def package_version(package_name: str) -> str:
    packages = {
        package["name"]: package["version"]
        for package in cargo_metadata()["packages"]
    }
    try:
        return packages[package_name]
    except KeyError:
        print(f"Package not found in Cargo metadata: {package_name}", file=sys.stderr)
        sys.exit(EXIT_ERROR)


def version_status(package_name: str, version: str) -> int:
    request = urllib.request.Request(
        f"https://crates.io/api/v1/crates/{package_name}/{version}",
        headers={"User-Agent": "rnk-release-workflow"},
    )

    try:
        with urllib.request.urlopen(request, timeout=30):
            print(f"{package_name} {version} exists")
            return EXIT_EXISTS
    except urllib.error.HTTPError as error:
        if error.code == 404:
            print(f"{package_name} {version} is missing")
            return EXIT_MISSING
        print(
            f"crates.io returned HTTP {error.code} for {package_name} {version}",
            file=sys.stderr,
        )
        return EXIT_ERROR
    except urllib.error.URLError as error:
        print(
            f"Could not check crates.io for {package_name} {version}: {error}",
            file=sys.stderr,
        )
        return EXIT_ERROR


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    package_version_parser = subparsers.add_parser("package-version")
    package_version_parser.add_argument("package")

    status_parser = subparsers.add_parser("version-status")
    status_parser.add_argument("package")
    status_parser.add_argument("version")

    args = parser.parse_args()

    if args.command == "package-version":
        print(package_version(args.package))
        return EXIT_EXISTS

    if args.command == "version-status":
        return version_status(args.package, args.version)

    parser.error(f"Unknown command: {args.command}")
    return EXIT_ERROR


if __name__ == "__main__":
    raise SystemExit(main())
