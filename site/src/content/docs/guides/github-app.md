---
title: Give sluice its own GitHub identity
description: A GitHub App for sluice, the only bypass of main's ruleset, so sluice's landings get through and every other push to main is refused, your agents' included.
sidebar:
  order: 3.5
---

With [main on GitHub](/sluice/guides/github/), sluice pushes each
landing with the credentials your own `git push` uses. Your agents run
git with those same credentials, so GitHub can't tell an agent's
`git push origin HEAD:main` from sluice's landing. This guide gives
sluice an identity of its own: a GitHub App. The app is the only bypass
of `main`'s ruleset, and only sluice's own fetches and pushes carry its
token.

## 1. Create the app

On GitHub, open **Settings › Developer settings › GitHub Apps › New
GitHub App**, in the organization that owns the repository if it has
one. Give it a name, such as `sluice for acme`, and any homepage URL.
Clear **Webhook › Active**: sluice asks GitHub, and GitHub never calls
sluice.

Under **Repository permissions**:

| permission | access |
|---|---|
| Contents | Read and write |
| Commit statuses | Read and write |
| Checks | Read and write |
| Pull requests | Read and write |
| Metadata | Read-only |

sluice needs contents to land. The statuses, checks and pull requests
are for [working through pull requests](/sluice/guides/pull-requests/).
Under **Where can this GitHub App be installed?** choose **Only on this
account**, then **Create GitHub App**. Note the **App ID** at the top
of the app's page.

## 2. Install it on the repository

On the app's page, **Install App**, choose the account, then **Only
select repositories** and pick the repository whose `main` sluice
lands on.

## 3. Keep its key on this machine

On the app's page, **Private keys › Generate a private key** downloads
a `.pem` file. Move it next to sluice's host config, where only you may
read it:

```console
$ mkdir -p ~/.config/sluice
$ mv ~/Downloads/sluice-for-acme.*.private-key.pem ~/.config/sluice/app.pem
$ chmod 600 ~/.config/sluice/app.pem
```

sluice refuses a key that anyone but you may read. The key never goes
in a repository; anyone who holds it can push as sluice.

## 4. Name the app in the host's config

In `~/.config/sluice/config.toml`:

```toml
[forge]
app_id = 123456                  # the App ID from step 1
key = "~/.config/sluice/app.pem"
```

From the next round, sluice makes a token from the key for the
repository, keeps it in memory, and gives it only to its own `git fetch`
and `git push`. Nothing is written to `.git/config`, so the git your
agents run never finds it. `sluice status` says who pushes:

```console
$ sluice status main
main at 7c829a5 · testing 0 at once
pushed to origin as GitHub app 123456
```

## 5. Make the app the ruleset's only bypass

In the repository's **Settings › Rules › Rulesets**, open the ruleset on
`main` from [Keep main in step with GitHub](/sluice/guides/github/#set-up-github).
Under **Bypass list**, **Add bypass**, choose the app, and set it to
**Always allow**. Remove every other entry, your own account and any
role included.

From now on a push to `main` from anyone but sluice is refused, yours
too. To land work, submit it.

## 6. Check it

```console
$ sluice forge check main
ok      [forge] key /home/me/.config/sluice/app.pem reads
ok      app 123456 makes a token for acme/widgets
ok      the app may write acme/widgets's contents
ok      ruleset "main" on main lets only app 123456 through
```

GitHub shows who may bypass a ruleset only to the repository's admins,
so `sluice forge check` reads the ruleset with your own token, from `gh`
or `GH_TOKEN`. It exits `1` and names what is wrong when anything is,
such as a second bypass left on the ruleset.

## What the app does not do

The key is a file your user may read, and an agent runs as your user.
The ruleset keeps agents off `main` because their git never uses the
key, not because they can't read it. Running agents as another user, or
keeping the key in a secrets manager, would close that gap; sluice
doesn't do either yet.
