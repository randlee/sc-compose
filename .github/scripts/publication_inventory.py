"""Read-only previous-publication inventory, with injectable service adapters."""
from __future__ import annotations

import base64
import re
import tomllib
from urllib.parse import quote

from publication_continuity import CHANNELS, ContinuityError, pypi_name
from release_immutability import ImmutabilityError, api
from release_manifest import registry_version_state


def version(value):
    match = re.fullmatch(r"v?(\d+)\.(\d+)\.(\d+)", value)
    return tuple(map(int, match.groups())) if match else None


class Inventory:
    def __init__(self, repository, *, query=api, probe=registry_version_state):
        self.repository = repository
        self.query = query
        self.probe = probe

    def get(self, path, *, absent=False):
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", self.repository):
            raise ContinuityError("repository must be OWNER/REPO")
        try:
            status, value = self.query(path)
        except ImmutabilityError as error:
            raise ContinuityError(f"previous-publication inventory unavailable: {error}") from error
        if absent and status == 404:
            return None
        if status != 200:
            raise ContinuityError(f"previous-publication inventory unavailable (HTTP {status})")
        return value

    def pages(self, path):
        separator = "&" if "?" in path else "?"
        for page in range(1, 101):
            rows = self.get(f"{path}{separator}per_page=100&page={page}")
            if not isinstance(rows, list) or any(not isinstance(row, dict) for row in rows):
                raise ContinuityError("invalid previous-publication listing")
            yield from rows
            if len(rows) < 100:
                return
        raise ContinuityError("previous-publication listing exceeded pagination limit")

    def read(self, path, ref, *, required=False, repository=None):
        location = f"repos/{repository or self.repository}/contents/{quote(path, safe='/')}?ref={quote(ref, safe='')}"
        item = self.get(location, absent=not required)
        if item is None:
            return None
        if not isinstance(item, dict) or item.get("encoding") != "base64" or not isinstance(item.get("content"), str):
            raise ContinuityError(f"invalid historical source file: {path}")
        try:
            return base64.b64decode("".join(item["content"].split()), validate=True).decode("utf-8")
        except (ValueError, UnicodeError) as error:
            raise ContinuityError(f"invalid historical source encoding: {path}") from error

    def published(self, url):
        try:
            state = self.probe(url)
        except SystemExit as error:
            raise ContinuityError(str(error)) from error
        if state not in {"published", "absent"}:
            raise ContinuityError("indeterminate previous registry publication")
        return state == "published"

    def previous(self, requested_version, manifest_path):
        current = version(requested_version)
        if current is None:
            raise ContinuityError("version must be X.Y.Z or vX.Y.Z")
        eligible = []
        notes = []
        for row in self.pages(f"repos/{self.repository}/releases"):
            if row.get("draft") is True or row.get("prerelease") is True:
                continue
            if row.get("draft") is not False or row.get("prerelease") is not False:
                raise ContinuityError("indeterminate previous release visibility")
            previous = version(row.get("tag_name", ""))
            if previous is None:
                if not isinstance(row.get("tag_name"), str) or not row["tag_name"]:
                    raise ContinuityError("previous release has no valid tag name")
                notes.append(f"Skipped non-stable-semver release tag: {row['tag_name']}")
                continue
            if previous < current:
                eligible.append((previous, row))
        if not eligible:
            return {"tag": None, "assets": [], "publications": [], "notes": notes}
        previous, release = max(eligible, key=lambda item: item[0])
        tag = release["tag_name"]
        if type(release.get("id")) is not int or release["id"] <= 0:
            raise ContinuityError("previous release has no valid asset inventory identifier")
        assets = []
        for row in self.pages(f"repos/{self.repository}/releases/{release['id']}/assets"):
            if not isinstance(row.get("name"), str) or not row["name"] or row.get("state") != "uploaded":
                raise ContinuityError("previous release has an incomplete asset")
            assets.append(row["name"])
        source = self.read(manifest_path, tag)
        if source is None:
            raise ContinuityError("previous release has no artifact manifest; registry/channel inventory cannot be established")
        manifest = tomllib.loads(source)
        publications = self.registries(manifest, tag.removeprefix("v"))
        publications.update(self.go_publications(previous))
        publications.update(self.other_channels(manifest))
        return {"tag": tag, "assets": assets, "publications": sorted(publications), "notes": notes}

    def registries(self, manifest, release_version):
        result = set()
        for row in manifest.get("crates", []):
            if row.get("publish") is True:
                name = row["package"]
                v = row.get("version", release_version)
                if self.published(f"https://crates.io/api/v1/crates/{quote(name, safe='')}/{quote(v, safe='')}"):
                    result.add(f"crate:{name}")
        if "pypi" in manifest.get("channels", {}):
            for row in manifest.get("python_distributions", []):
                name = pypi_name(row["name"])
                v = row.get("version", release_version)
                if self.published(f"https://pypi.org/pypi/{quote(name, safe='')}/{quote(v, safe='')}/json"):
                    result.add(f"pypi:{name}")
        if "npm" in manifest.get("channels", {}):
            for row in manifest.get("npm_packages", []):
                name = row["name"]
                v = row.get("version", release_version)
                if self.published(f"https://registry.npmjs.org/{quote(name, safe='')}/{quote(v, safe='')}"):
                    result.add(f"npm:{name}")
        return result

    def go_publications(self, previous):
        """Only tags for the immediately previous version belong to the baseline."""
        modules = {}
        for row in self.pages(f"repos/{self.repository}/tags"):
            name = row.get("name")
            if not isinstance(name, str):
                raise ContinuityError("invalid historical tag listing")
            match = re.fullmatch(r"(?:(.+)/)?v(\d+\.\d+\.\d+)", name)
            if not match:
                continue
            source, number = match.groups()
            parsed = version(number)
            if parsed != previous:
                continue
            source = source or ""
            if source not in modules or modules[source][0] < parsed:
                modules[source] = (parsed, name)
        result = set()
        for source, (_, tag) in modules.items():
            path = f"{source}/go.mod" if source else "go.mod"
            text = self.read(path, tag)
            if text is None:
                continue
            matches = re.findall(r"^module\s+(\S+)\s*$", text, re.MULTILINE)
            if len(matches) != 1:
                raise ContinuityError(f"invalid published Go module at {tag}")
            result.add(f"go:{matches[0]}")
        return result

    def other_channels(self, manifest):
        channels = manifest.get("channels", {})
        unsupported = set(channels) - CHANNELS
        if unsupported:
            raise ContinuityError(f"no previous-publication provider for {sorted(unsupported)}")
        result = set()
        if "homebrew" in channels:
            channel = channels["homebrew"]
            for row in channel["formulas"]:
                if self.read(row["path"], "HEAD", repository=channel["tap_repository"]) is not None:
                    result.add(f"homebrew:{channel['tap_repository']}/{row['path']}")
        if "scoop" in channels:
            channel = channels["scoop"]
            if self.read(channel["manifest_path"], "HEAD", repository=channel["bucket_repository"]) is not None:
                result.add(f"scoop:{channel['bucket_repository']}/{channel['manifest_path']}")
        if "winget" in channels:
            identifier = channels["winget"]["identifier"]
            path = "manifests/" + identifier[0].lower() + "/" + identifier.replace(".", "/")
            # A package directory proves a published identifier; version files
            # may already have advanced on a retry of the new release.
            item = self.get(f"repos/microsoft/winget-pkgs/contents/{quote(path, safe='/')}", absent=True)
            if item is not None:
                if not isinstance(item, list) or not item:
                    raise ContinuityError("invalid previous Winget publication listing")
                result.add(f"winget:{identifier}")
        return result

