# `setup-nub`

A GitHub Action that installs the [nub](https://github.com/nubjs/nub) CLI on a CI runner and puts it on `PATH`. It lives in the main nub repository, so you reference it by subpath.

## Usage

```yaml
- uses: nubjs/nub/setup@v0
  with:
    version: latest
- run: nub install
```

Pin a specific version:

```yaml
- uses: nubjs/nub/setup@v0
  with:
    version: 0.0.44
```

The `version` input accepts any range npm understands (`0.0.44`, `^0.0`, `latest`).

## Inputs

| Input     | Default  | Description                                                              |
| --------- | -------- | ------------------------------------------------------------------------ |
| `version` | `latest` | The nub version to install — any semver range npm accepts.               |

## Outputs

| Output    | Description                                                              |
| --------- | ------------------------------------------------------------------------ |
| `version` | The installed nub version, as reported by `nub --version` (`v<semver>`). |

```yaml
- uses: nubjs/nub/setup@v0
  id: nub
- run: echo "Installed ${{ steps.nub.outputs.version }}"
```

## How it works

The action runs `npm install -g @nubjs/nub@<version>`. npm is preinstalled on every GitHub-hosted runner, so there is no bootstrap step. The install pulls the matching `@nubjs/nub-<platform>` optional dependency for the runner's OS and architecture, links the `nub` and `nubx` bins into the npm global bin directory — already on `PATH` on `ubuntu-latest`, `macos-latest`, and `windows-latest` — and runs nub's postinstall to set the binary's execute bit.

## Supported runners

Works on `ubuntu-latest`, `macos-latest`, and `windows-latest`. The action uses a `bash` step; GitHub-hosted Windows runners ship Git Bash, so the same step runs on all three.

## Caching

Not yet. Each run does a fresh global install. Caching the npm global prefix is tracked as a follow-up.
