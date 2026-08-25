---
format: aep.planning-md/1
id: epic:declared-configuration-invariants
kind: epic
status: draft
title: The configuration a project depends on is declared, not discovered by breaking
summary: Every environment variable a project reads, with what it is for, its default and whether it may be absent — found from the source, declared in a schema, and joined against what the cluster actually supplies.
owner: protocol
tags:
- configuration
- infra
- invariants
relations:
- decomposes: initiative:the-repo-governs-itself
revision: 2
---
# Epic: The configuration a project depends on is declared, not discovered by breaking

## Outcome

A project can be asked *what configuration do you require* and answer from a document: every
environment variable its code reads, what the variable is for, what it defaults to, whether it may
be absent, and what happens when it is wrong. The answer is derived from the source rather than
maintained by hand, and it can be checked against what a running deployment actually supplies.

## Why Now

Configuration is the largest body of undeclared invariants in most repositories. `DATABASE_URL`
must be set; `LOG_LEVEL` defaults to `info`; `FEATURE_X` is read in two places that disagree about
its default — none of that is written anywhere a tool can read, and all of it is load-bearing. It is
learned by breaking something, usually in the environment where breaking is most expensive.

This is the same meta-defect as `story:open-vocabulary-audit`, on a different surface: a thing that
is decisive is fixed somewhere nobody thought to write down. That audit's rule applies here too —
the output is not *declare everything*, it is that nothing decisive is undeclared **by accident**.

Half of the join already exists and points at the gap. `crates/infra-domain/src/workload.rs:108`
models an `EnvVar { name, source }` with an `EnvSource` that distinguishes a literal from a
reference — but that is the **deployed** side, read out of a cluster by `infra-scout` and parsed
from an `infra-observation/1` bundle. Nothing models the **source** side: what the code reads, and
what it means. The two sides are a requirement and its evidence, and joining them is the point:

* a variable the code requires and the cluster does not supply is a deployment that will fail, and
  nothing today says so before it does;
* a variable the cluster supplies and no code reads is dead configuration nobody dares delete;
* a variable whose declared default disagrees with the code's is a document that lies.

## Scope

* **Discovery, per language.** Finding what a project reads is language-specific — `std::env::var`
  in Rust, `os.Getenv` in Go, `process.env` in TypeScript, `os.environ` in Python. A project already
  declares its language; the scanner is selected from that.
* **A schema for the declaration.** Name, purpose, default, required-or-not, and the shape of a
  legal value. The schema is the artifact; discovery seeds it and does not own it, because *what it
  is for* is knowledge no scanner has.
* **The join against `infra-observation/1`**, which is what turns a declaration into a checkable
  claim rather than a nicer README.
* **Refusing silently.** A variable found by the scanner and absent from the declaration must be a
  finding, in both directions, or the document rots the way every configuration document rots.

## Out of Scope

* Reading secret **values**. `infra-observation/1` already replaces every secret with
  `{sha256, length}` and that boundary is not moved here.
* Non-environment configuration — files, flags, service discovery. The same argument applies to each
  and each is its own epic; environment variables are the one with a deployed side already modelled.
* Managing configuration. This declares and checks; it does not set, template or deploy.

## Risks

* **A scanner that is nearly right is worse than none.** A dynamic read — a name assembled at run
  time, a variable read through a wrapper — is invisible to a scanner, and a declaration that
  presents itself as complete when it is not is exactly the false confidence this epic exists to
  remove. Whatever ships must state what it cannot see, the way `docs/guide/open-vocabulary.md`
  states that its derivation cannot discover a closed surface.
* **The metadata is the expensive half and no tool produces it.** Purpose and default are written by
  a person. An epic that ships discovery and calls it done delivers a list of names.
* **Language coverage grows without end.** Deciding which languages are in and saying so beats
  implying all of them.

## Done When

A repository declares its configuration in a document this toolchain validates; the declaration is
seeded from its own source and drift in either direction is a finding; and a deployment observation
can be checked against it, so *this service is missing a variable it requires* is answerable before
the service is started rather than after.
