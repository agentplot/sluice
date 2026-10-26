# Forge identity: a GitHub App

**Choose it when** a branch has `remote` on GitHub and agents run git with
the user's credentials. The app is the ruleset's only bypass, and only
sluice's own fetches and pushes carry its token, so every other push to the
branch, agents' included, is refused.

These steps are on GitHub and in the user's host config: walk the user
through them; do not do them unasked.

## 1. Create the app

**Settings › Developer settings › GitHub Apps › New GitHub App** (in the
organization that owns the repository, if any). Clear **Webhook › Active**.
Repository permissions:

| permission | access | why |
|---|---|---|
| Contents | Read and write | landing |
| Workflows | Read and write | landing a change under `.github/workflows` |
| Commit statuses | Read and write | pull requests |
| Checks | Read and write | the `sluice` check on pull requests |
| Pull requests | Read and write | opening and commenting on pull requests |
| Metadata | Read-only | required |

**Only on this account**, then create. Note the App ID.

## 2. Install it on the repository

**Install App** › the account › **Only select repositories** › the repository.

## 3. Keep its key on the host

**Private keys › Generate a private key**, then:

```sh
mkdir -p ~/.config/sluice
mv ~/Downloads/<app>.*.private-key.pem ~/.config/sluice/app.pem
chmod 600 ~/.config/sluice/app.pem
```

sluice refuses a key anyone but the owner may read. Never commit it.

## 4. Name it in the host config

`~/.config/sluice/config.toml`, once the skill's step 6, *The host's file*,
has said who writes it; where a tool manages the host's configuration, the
user adds it there and keeps the key out of any store everyone may read:

```toml
[forge]
app_id = 123456
key = "~/.config/sluice/app.pem"
```

Nothing is written to `.git/config`; the token lives in sluice's memory and
reaches only its own git.

## 5. Make it the ruleset's only bypass

In the branch's ruleset, **Bypass list › Add bypass**, the app, **Always
allow**. Remove every other entry.

## 6. Check

```sh
sluice forge check main
```

It reads the rulesets with the user's own token (`GH_TOKEN`, `GITHUB_TOKEN`
or `gh auth token`) and everything else as the app, prints `ok` or `failed`
per item, and exits `1` naming what is wrong.

## Limits

The key is a file the user's account may read, and agents run as that user:
the ruleset holds because agents' git never uses the key, not because they
cannot read it.

Reference: <https://agentplot.github.io/sluice/guides/github-app/>.
