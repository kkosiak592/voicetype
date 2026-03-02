---
phase: quick-22
plan: 01
subsystem: profiles
tags: [structural-engineering, corrections, whisper, vocabulary]
dependency_graph:
  requires: []
  provides: [expanded-structural-engineering-profile]
  affects: [transcription-accuracy]
tech_stack:
  added: []
  patterns: [corrections-hashmap]
key_files:
  created: []
  modified:
    - src-tauri/src/profiles.rs
decisions:
  - Preserved all 13 existing corrections unchanged; plan stated 12 but file had 13
  - Final total is 55 correction entries (13 original + 42 new), plan expected 54
metrics:
  duration: ~3 minutes
  completed: "2026-03-02"
  tasks_completed: 1
  files_modified: 1
---

# Phase quick-22 Plan 01: Add 42 New Structural Engineering Corrections Summary

Added 42 new correction entries and expanded the initial_prompt in the built-in structural_engineering_profile, bringing the total from 13 to 55 corrections covering shear misrecognitions, code abbreviations (AISC/ACI/ASCE/AASHTO), component terms, named concepts, rebar sizes, and software names.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add 42 correction entries and expand initial_prompt | 7ce36f0 | src-tauri/src/profiles.rs |

## What Was Built

- **6 shear corrections:** sheer/sheer wall/sheer force/sheer stud/sheer connection/punching sheer -> shear variants
- **1 shape correction:** why shape -> W-shape
- **14 code/standard abbreviation corrections:** a disc -> AISC, a see I -> ACI, a see E -> ASCE, Osh toe/ash toe -> AASHTO, Alfred -> LRFD, E tabs -> ETABS, aisc 360/three sixty -> AISC 360, aisc 341/three forty one -> AISC 341, asce 7/asce seven -> ASCE 7, a disc 360 -> AISC 360
- **8 component/material corrections:** rebirth/re bar -> rebar, gust it -> gusset, stiffen her -> stiffener, fill it weld -> fillet weld, stir up -> stirrup, flex your -> flexure, lentil -> lintel
- **3 named concept corrections:** Oiler buckling -> Euler buckling, Moore's circle -> Mohr's circle, poison's ratio -> Poisson's ratio
- **2 technique corrections:** pre stressed -> prestressed, post tensioning -> post-tensioning
- **6 rebar size corrections:** number 3/6/7/8/9/10 bar -> #3/#6/#7/#8/#9/#10 bar
- **2 software corrections:** stood pro -> STAAD Pro, sap 2000 -> SAP2000
- **Expanded initial_prompt** with 20 new domain terms: W-shape, shear, gusset plate, stiffener, LRFD, ASD, AASHTO, ASCE, post-tensioning, axial, flexure, buckling, diaphragm, splice, ksi, DCR, ETABS, SAP2000, stirrup, lintel, fillet weld

## Verification

- `cargo check` passed with no errors
- 55 total `corrections.insert()` calls confirmed in structural_engineering_profile()
- All existing entries preserved unchanged

## Deviations from Plan

### Minor Discrepancy — Entry Count

The plan stated 12 existing corrections and 54 total (12 + 42). The file actually had 13 existing corrections, resulting in 55 total. All 42 new entries were added as specified and all 13 original entries preserved unchanged. The plan's count was off by one (it apparently overlooked one of the original entries).

## Self-Check: PASSED

- File exists: src-tauri/src/profiles.rs — FOUND
- Commit 7ce36f0 exists — FOUND
- `punching shear` present in file — FOUND (via punching sheer -> punching shear correction)
- cargo check passes — CONFIRMED
