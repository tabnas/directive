/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */

//! Shared test scaffolding: the mini host grammar the directive plugin is
//! layered on, and the runner for the shared `test/spec/*.tsv` fixtures.
//!
//! Cargo compiles this module into EVERY integration test binary, so a
//! helper only one of them uses reads as dead code in the others —
//! `doc_examples_test` needs the grammar but not the spec runner. The
//! allow is about that compilation model, not about unused code.

#![allow(dead_code)]

pub mod mini_grammar;
pub mod spec;
