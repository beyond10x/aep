---
title: "0.23.1 — the pin says what it links, and the ladder says draft"
description: >
  Two corrections that are the same mistake in two places: a dependency pin recording a tag two
  releases behind what exists, and a new ladder starting at drafted — one letter from a built-in.
slug: the-pin-says-what-it-links
tags: [release, aep, planning]
---

A small release, and both fixes are the same mistake wearing different clothes: **a label that had
drifted from the thing it names.**

{/* truncate */}

## The pin

`aep-backend-markdown` pinned `entity-core` at a tag **two releases behind what existed**.

Nothing this crate links actually moved — the source is byte-identical between the two tags, because
the releases in between carried an examples-only change. So this changed no behaviour at all.

It was still worth fixing, for a reason that is not about this pin: **a pin stale by label is a pin
nobody trusts to be current by content either.** The next person reading it has to go and check
whether the drift matters, every time, which is exactly the work a pin exists to save.

## The ladder

`outbound-claim` started at `drafted`, not `draft`.

Every other shipped ladder starts at the built-in `draft`. An invented rung **one letter from a
built-in** is a typo wearing a vocabulary's clothes — and it would have been inherited by everybody
who copied the file.

This is the boundary the open vocabulary was designed to hold. **The vocabulary is open to authors,
not to near-misses.** A status is accepted because *some ladder declares it*, never because it
parses — but a ladder can still declare something its author did not mean to, and the first shipped
example doing so is the one that teaches it to everybody else.
