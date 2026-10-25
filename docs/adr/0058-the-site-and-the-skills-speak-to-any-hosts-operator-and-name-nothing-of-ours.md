---
number: 58
title: The site and the skills speak to any host's operator, and name nothing of ours
status: accepted
date: 2026-10-26
decision-makers:
- Chuck Swanberg
---

# The site and the skills speak to any host's operator, and name nothing of ours

Status: accepted by the user in madswan-design's pane on 2026-10-26 ("People aren't going to know what the word "keeper" means or anything about SwanCloud machine configuration. This needs to be presented in a more generic way").

Deciders: the user, on a documentation chore whose words said "the keeper" and "swancloud's machine configuration", in madswan-design's pane on 2026-10-26, as quoted above; the record's other words are madswan-design's.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 35, 71 (which coin "the keeper"), and its givens ("sluice is free and open source"); rules core.1, core.13; the setting-up skill's step 6; `site/src/content/docs`.

## Context and problem statement

The statement writes for the family: it names the keeper, the host's shared caches it declares, the fleet that writes them. A chore to document the host's settings carried those words onto the site and into the setup skill, where a reader is anyone who installed sluice on a machine and knows nothing of our fleet, of swancloud, or of what the statement means by a keeper. Which words may the product's own pages use?

## Considered options

1. The site, the README, the plugins' skills and the commands' output speak to any host's operator: "whoever administers the host", "configuration management (Nix, Ansible, an MDM)", and never a word the statement coined that the reader cannot see in the product, nor a machine, fleet, partition or tool of ours.
2. The product's pages share the statement's vocabulary, with a glossary.
3. No rule; each writer judges.

## Decision outcome

Option 1. A glossary teaches a reader our words so that our pages can keep them, which is backwards: the product's pages exist for the reader. The statement and the records keep their vocabulary, since they are written for whoever builds sluice, and a rule is drawn from this record so that no chore carries the words across again. A host's settings file is described as written either by the person administering the host or by whatever manages the host's configuration, and the setup skill, finding the file managed, read-only or a link into a store, says to change it where it is managed, naming no manager of ours.

### Consequences

- Rule core.26 is drawn from this record.
- The chore `the-site-and-the-setup-skill-document-the-hosts-settings-and-who-writes-them` is reworded before it is built.
- The statement's "keeper" stays the statement's word.
