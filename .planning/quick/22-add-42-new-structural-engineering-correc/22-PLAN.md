---
phase: quick-22
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/profiles.rs
autonomous: true
requirements: [QUICK-22]
must_haves:
  truths:
    - "All 42 new correction entries exist in the structural_engineering_profile corrections HashMap"
    - "The initial_prompt includes all specified additional domain terms"
    - "Existing 12 correction entries are preserved unchanged"
    - "Project compiles without errors"
  artifacts:
    - path: "src-tauri/src/profiles.rs"
      provides: "Expanded structural engineering profile with 54 total corrections"
      contains: "punching shear"
  key_links: []
---

<objective>
Add 42 new structural engineering correction dictionary entries to the built-in structural_engineering_profile() in src-tauri/src/profiles.rs and expand the initial_prompt with additional domain terms.

Purpose: Improve transcription accuracy for structural engineering terminology by catching more spoken approximations and biasing Whisper toward domain vocabulary.
Output: Updated profiles.rs with 54 total corrections (12 existing + 42 new) and an expanded initial_prompt.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/profiles.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add 42 correction entries and expand initial_prompt</name>
  <files>src-tauri/src/profiles.rs</files>
  <action>
In `structural_engineering_profile()` in src-tauri/src/profiles.rs, add these 42 new `corrections.insert()` calls AFTER the existing 12 entries. Group them with comments for readability.

**Shear-related corrections:**
```rust
// Shear corrections (common Whisper misrecognition of "shear")
corrections.insert("sheer".to_string(), "shear".to_string());
corrections.insert("sheer wall".to_string(), "shear wall".to_string());
corrections.insert("sheer force".to_string(), "shear force".to_string());
corrections.insert("sheer stud".to_string(), "shear stud".to_string());
corrections.insert("sheer connection".to_string(), "shear connection".to_string());
corrections.insert("punching sheer".to_string(), "punching shear".to_string());
```

**Shape/section corrections:**
```rust
// Shape and section corrections
corrections.insert("why shape".to_string(), "W-shape".to_string());
```

**Code/standard abbreviation corrections:**
```rust
// Code and standard abbreviation corrections
corrections.insert("a disc".to_string(), "AISC".to_string());
corrections.insert("a see I".to_string(), "ACI".to_string());
corrections.insert("a see E".to_string(), "ASCE".to_string());
corrections.insert("Osh toe".to_string(), "AASHTO".to_string());
corrections.insert("ash toe".to_string(), "AASHTO".to_string());
corrections.insert("Alfred".to_string(), "LRFD".to_string());
corrections.insert("E tabs".to_string(), "ETABS".to_string());
corrections.insert("aisc 360".to_string(), "AISC 360".to_string());
corrections.insert("aisc three sixty".to_string(), "AISC 360".to_string());
corrections.insert("aisc 341".to_string(), "AISC 341".to_string());
corrections.insert("aisc three forty one".to_string(), "AISC 341".to_string());
corrections.insert("asce 7".to_string(), "ASCE 7".to_string());
corrections.insert("asce seven".to_string(), "ASCE 7".to_string());
corrections.insert("a disc 360".to_string(), "AISC 360".to_string());
```

**Component/material corrections:**
```rust
// Component and material corrections
corrections.insert("rebirth".to_string(), "rebar".to_string());
corrections.insert("re bar".to_string(), "rebar".to_string());
corrections.insert("gust it".to_string(), "gusset".to_string());
corrections.insert("stiffen her".to_string(), "stiffener".to_string());
corrections.insert("fill it weld".to_string(), "fillet weld".to_string());
corrections.insert("stir up".to_string(), "stirrup".to_string());
corrections.insert("flex your".to_string(), "flexure".to_string());
corrections.insert("lentil".to_string(), "lintel".to_string());
```

**Named concept corrections:**
```rust
// Named concept corrections
corrections.insert("Oiler buckling".to_string(), "Euler buckling".to_string());
corrections.insert("Moore's circle".to_string(), "Mohr's circle".to_string());
corrections.insert("poison's ratio".to_string(), "Poisson's ratio".to_string());
```

**Technique/method corrections:**
```rust
// Technique and method corrections
corrections.insert("pre stressed".to_string(), "prestressed".to_string());
corrections.insert("post tensioning".to_string(), "post-tensioning".to_string());
```

**Rebar size corrections:**
```rust
// Rebar size corrections
corrections.insert("number 3 bar".to_string(), "#3 bar".to_string());
corrections.insert("number 6 bar".to_string(), "#6 bar".to_string());
corrections.insert("number 7 bar".to_string(), "#7 bar".to_string());
corrections.insert("number 8 bar".to_string(), "#8 bar".to_string());
corrections.insert("number 9 bar".to_string(), "#9 bar".to_string());
corrections.insert("number 10 bar".to_string(), "#10 bar".to_string());
```

**Software corrections:**
```rust
// Software name corrections
corrections.insert("stood pro".to_string(), "STAAD Pro".to_string());
corrections.insert("sap 2000".to_string(), "SAP2000".to_string());
```

Then update the `initial_prompt` string to include the additional terms. Replace the existing initial_prompt with:
```rust
initial_prompt: "I-beam, W-section, W-shape, W8x31, MPa, rebar, AISC, ACI 318, kips, PSI, \
    prestressed concrete, shear wall, moment frame, deflection, compressive strength, \
    tensile strength, grade 60 rebar, shear, gusset plate, stiffener, LRFD, ASD, \
    AASHTO, ASCE, post-tensioning, axial, flexure, buckling, diaphragm, splice, \
    ksi, DCR, ETABS, SAP2000, stirrup, lintel, fillet weld"
    .to_string(),
```

IMPORTANT: Preserve ALL 12 existing corrections entries exactly as they are. Only ADD new entries after them.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && cargo check 2>&1 | tail -5</automated>
  </verify>
  <done>
- All 42 new correction entries are present in structural_engineering_profile()
- All 12 original entries are preserved unchanged
- initial_prompt includes all 20 new domain terms
- `cargo check` passes with no errors
  </done>
</task>

</tasks>

<verification>
1. `cargo check` compiles without errors
2. Count total corrections.insert() calls in structural_engineering_profile() — should be 54 (12 existing + 42 new)
3. initial_prompt contains all added terms: shear, gusset plate, stiffener, LRFD, ASD, AASHTO, ASCE, post-tensioning, axial, flexure, buckling, diaphragm, splice, ksi, DCR, ETABS, SAP2000, stirrup, lintel, fillet weld
</verification>

<success_criteria>
- profiles.rs compiles cleanly
- 54 total correction entries in structural_engineering_profile()
- Expanded initial_prompt with all specified domain terms
- No existing entries modified or removed
</success_criteria>

<output>
After completion, create `.planning/quick/22-add-42-new-structural-engineering-correc/22-SUMMARY.md`
</output>
