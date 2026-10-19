# Releasing

Three workflows release GRKit:

- `Release PR` keeps one pull request from the `release` branch open while change files are pending. knope sets the version in `Cargo.toml` and `Cargo.lock` and writes `CHANGELOG.md` from the change files.
- `Release` starts when the release pull request is merged. It tags the merge commit, runs the tests, and builds the site. Then it creates a draft GitHub release that holds the site and the device-test packages.
  To build the site, it downloads the official GR III 2.10 and GR IV 1.11 firmware and checks their SHA-256. Then it builds a runtime for each with LLVM 21, publishes the effects in `effects/` with their preview tables, and deletes the firmware. It also builds the web page in `web/`.
- `Deploy` commits the site of one draft release to the `site` branch. Cloudflare Pages deploys each commit on that branch. When the site serves the new version, `Deploy` publishes the release. The deployed bytes are therefore the tested bytes.

The `site` branch holds only deployed sites, one commit for each deployment. It shares no history with `main`. Do not push to it by hand. The repository holds no Cloudflare credentials, because Cloudflare reads the branch through its GitHub app.

The version is the `[workspace.package]` version in `Cargo.toml`. Each runtime `build.json` gives it, and the site shows it in the footer. A release tag is `v` and the version, for example `v0.1.0`.

`grkit site` writes only `build.json` and `sd/` to `site/runtime/<target>/`, for `gr3-2.10` and `gr4-1.11`. It leaves out `objects/`, because the generated header there holds firmware bytes.
`grkit effects publish` writes the v1 catalog, `.GRF` files, and preview tables to `site/effects/<target>/` for both targets.
The tables include `preview/standard.lut` and `preview/<id>.lut`.

`.github/scripts/build-guides.sh` writes the installation guides to `site/guides/<camera>/<language>.pdf` with Typst. `.github/scripts/install-guide-tools.sh` pins the Typst version and the fonts. The web build adds `index.html`, `assets/`, `samples/`, and `_headers`.

## Set up

1. In the repository settings, under Actions, General, select "Allow GitHub Actions to create and approve pull requests". `Release PR` needs it.
2. If the repository has no `site` branch, push an orphan `site` branch with one placeholder `index.html`. Cloudflare can connect only to a branch that exists.
3. Connect a Cloudflare Pages project to the repository:
   1. In the Cloudflare dashboard, go to Workers & Pages. Select Create, Pages, and Connect to Git.
   2. Install the Cloudflare GitHub app for this repository only.
   3. Set the production branch to `site` and the framework preset to None. Leave the build command empty. Set the build output directory to `/`.
   4. After Cloudflare creates the project, go to Settings, Builds, Branch control. Set the preview branch to None. Otherwise Cloudflare deploys the source of each other branch as a preview.
4. Add the repository variable `SITE_URL` with the production URL of the site and no trailing slash, for example `https://<project>.pages.dev`. `Deploy` reads `SITE_URL/runtime/gr3-2.10/build.json` to find the version that the site serves.
5. Keep a private backup of the firmware, so that a release does not depend on the Ricoh download:
   1. Download `gr3_v210.zip` and `gr4_v111.zip` from Ricoh and accept the license. Do not change the zips.
   2. Create a private repository: `gh repo create <owner>/<backup-repo> --private`.
   3. Upload each zip as a release asset with the tag of its target:
      - `gh release create gr3-2.10 gr3_v210.zip --repo <owner>/<backup-repo> --notes "Private backup. Do not redistribute."`
      - `gh release create gr4-1.11 gr4_v111.zip --repo <owner>/<backup-repo> --notes "Private backup. Do not redistribute."`
   4. Create a fine-grained token that can read only the Contents of that repository.
   5. In this repository, add the secret `FIRMWARE_BACKUP_TOKEN` with the token, and the variable `FIRMWARE_BACKUP_REPO` with `<owner>/<backup-repo>`.
   6. Start the `Release` workflow with an existing release tag and `firmware_source` set to `backup`. If the `build` job succeeds, the backup works.

The backup must stay private. The Ricoh license permits a copy only to save and back up the software.
With `firmware_source` set to `auto`, the build uses the backup of a zip only if the Ricoh download of that zip fails or its SHA-256 differs. Both sources must have the same SHA-256.

## Record a change

Each change that a user can see needs a change file in `.changeset/`. Add it in the commit of the change. Write it with `knope document-change`, or by hand:

```md
---
default: patch
---

Show the GRKit version in the site footer.
```

Write the text in English, for people who use GRKit. The change type sets the version and the `CHANGELOG.md` section:

| Type | Section | Below 1.0 | From 1.0 |
| --- | --- | --- | --- |
| `patch` | Fixes | 0.1.0 to 0.1.1 | 1.0.0 to 1.0.1 |
| `minor` | Features | 0.1.0 to 0.1.1 | 1.0.0 to 1.1.0 |
| `major` | Breaking Changes | 0.1.0 to 0.2.0 | 1.0.0 to 2.0.0 |

Use `major` for a change that an earlier package cannot follow, for example a new format of `STATE.TXT`, `PLUGINS.TXT`, or the GRF files.

## Release a version

1. Merge the `Release vX.Y.Z` pull request. `Release` starts on the merge commit.
2. When the run is complete, open the draft release `vX.Y.Z`. It holds `site.tar.gz`, `GRKit-device-test-gr3.zip`, `GRKit-device-test-gr3x.zip`, and `GRKit-device-test-gr4.zip`. Each package has the published runtime and no effects.
3. Install the package of each camera that you have, and test it. The GR IV runtime stays experimental until it passes a test on a camera.
4. If the test passes, start the `Deploy` workflow with the tag `vX.Y.Z`. The run waits up to 15 minutes for Cloudflare Pages. If the site does not serve the version in that time, the run fails and the release stays a draft. Examine the deployment in the Cloudflare dashboard. If it failed, retry it there, because a second `Deploy` run makes no new commit. Then start `Deploy` again.

To release a commit on `main` without a release pull request, push a tag `vX.Y.Z` that matches the version in `Cargo.toml`. The first release, `v0.1.0`, uses this path.
To build a tag again, for example with `firmware_source` set to `backup`, start `Release` with that tag. A published release is never replaced.

The draft release keeps its files without an expiry date. To go back to an earlier version, start `Deploy` with its tag.

The release toolchain is LLVM 21 on Ubuntu, not the reference Apple Clang. Test each CI runtime on a camera before you deploy it.
