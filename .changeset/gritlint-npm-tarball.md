---
"@systemfsoftware/gritlint": minor
---

gritlint now ships as an npm-style tarball that the flake builds as its `gritlint-tarball` output. A project installs it from the tarball file, with no registry and no network. The tarball carries static binaries for linux-x64 and linux-arm64 and a node launcher that runs the one for the host. On any other platform, `gritlint` exits 2 and names the two platforms it supports.
