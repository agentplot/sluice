# landing-subject Specification

## Purpose
TBD - created by archiving change a-landing-pull-request-is-titled-by-what-lands. Update Purpose after archive.

## Requirements

### Requirement: A landing commit names the work by its subject or its branch
A submission SHALL keep the subject its submit gave, in the branch's
records. The landing commit of a submission SHALL name its work, the
`{from}` of the policy's `[landing] subject`, by that subject, as
written, or by the submitted branch where the submit gave none; its
`Sluice-From` trailer SHALL still name the agent and the branch. A
subject on a branch that takes no pull requests is so neither refused
nor lost (23).

#### Scenario: A subject on a branch with no pull requests
- **WHEN** an agent on `unit/review` runs `sluice submit bolt/x --subject "Batch review cards"`, `bolt/x` has no `pull_requests` and the default `[landing] subject`, and the submission lands
- **THEN** the submit printed no warning about its subject, and the landing commit's first line is `chore(land): #<id> Batch review cards` and its trailers hold `Sluice-From: <agent> (unit/review)`

#### Scenario: No subject
- **WHEN** the same agent runs `sluice submit bolt/x` with no subject and the submission lands
- **THEN** the landing commit's first line is `chore(land): #<id> unit/review`, as before

#### Scenario: A policy's own landing subject
- **WHEN** `bolt/x`'s policy sets `[landing] subject = "land {from} on {to}"` and a submission given `--subject "Batch review cards"` lands
- **THEN** the landing commit's first line is `land Batch review cards on bolt/x`

#### Scenario: A submission from before subjects were kept
- **WHEN** a submission recorded with no subject field lands
- **THEN** its landing commit is named by its branch, and the record is read as before

### Requirement: A submission taken from a pull request keeps its title
A submission sluice takes from a pull request SHALL keep the request's
title as its subject, so its landing commit names the request's work
by its title (23, 118, 120).

#### Scenario: A request sluice opened with a subject
- **WHEN** an agent's `sluice submit main --subject "Batch review cards"` opened a pull request from `unit/review`, `main` has `pull_requests`, and the request is taken and lands
- **THEN** the landing commit's first line is `chore(land): #<id> Batch review cards` and its trailers still name the request's place, `pull/<n>`

#### Scenario: A request a person opened
- **WHEN** a person opened a pull request titled `Review cards, first pass` and it is taken and lands
- **THEN** the landing commit's first line is `chore(land): #<id> Review cards, first pass`
