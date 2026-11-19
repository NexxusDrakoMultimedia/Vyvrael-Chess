# NNUE

This document will describe Vyvræl's neural evaluation: architecture, input
features, quantisation, file format, training recipe and how to reproduce a
net. It is updated in the same commit as any network-related change.

## Current status

**Nothing is implemented.** Neural evaluation is milestone 8. There is no
network, no trainer code and no inference code. Everything below is the plan
of record, to be replaced by exact specifications as each part lands.

## Planned architecture

- A single-hidden-layer perspective network: `768 -> N (x2 perspectives) -> 1`.
- **Input features (768):** one input per (piece colour, piece type, square)
  = 2 x 6 x 64, computed once from each side's perspective (the board is
  mirrored for the side to move's opponent).
- **Activation:** clipped ReLU or SCReLU; chosen by experiment and recorded
  here.
- **Hidden size `N`:** to be decided by experiment and recorded here.

## Planned inference

- Quantised integer inference.
- Accumulators updated incrementally on make/unmake.
- Tests: the incremental accumulator always equals a from-scratch recompute;
  network load/save round-trips.
- The hand-crafted evaluation stays selectable through the `UseNNUE` UCI
  option so the two can be compared by SPRT. The network is loaded through the
  `EvalFile` option.

## Quantisation scheme

To be specified with the implementation (scale factors, integer widths,
clipping bounds).

## Binary file format

To be specified with the implementation (header, endianness, layer order,
padding).

## Training recipe

- **Trainer:** `bullet` (https://github.com/jw1912/bullet), from
  `tools/vyvrael-train`. Its licence must be checked and recorded in
  [DEPENDENCIES.md](DEPENDENCIES.md) before it is added.
- **Data:** self-play positions from Vyvræl's `datagen` (position, search
  score, game result), with per-game and per-thread seeds derived from the
  base seed `0x31031996` (overridable with `--seed`).
- **Hyperparameters and seeds:** recorded per net in its manifest.

## Reproducing a net

To be written with the first net: the exact datagen command, training
command, seeds and expected checksums.

## Net manifest format

Each trained net gets a manifest committed next to its documentation (the
net file itself is not committed unless it is small and the maintainer
approves). The manifest will record at least: net name and checksum,
architecture, activation, quantisation, trainer version, dataset description
and size, datagen parameters, all hyperparameters, all seeds, and the SPRT
result that justified adopting the net. The concrete format is specified when
the first net is trained.

## Acceptance

A net is adopted only after SPRT shows a gain over the hand-crafted
evaluation or the previous net. The result is recorded in the manifest and in
[TESTING.md](TESTING.md).
